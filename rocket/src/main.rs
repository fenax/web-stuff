#[macro_use]
extern crate rocket;

use lazy_static::*;

use rocket::http::{CookieJar, Cookie};
use rocket::fairing::{self, AdHoc};
use rocket::form::{ Form};
use rocket::{Build, Rocket, State};

use migration::{MigratorTrait};
use sea_orm::{EntityTrait, ActiveModelTrait, Set, Unchanged, QueryFilter, ColumnTrait};
use sea_orm_rocket::{Connection, Database};

mod pool;
use pool::Db;

pub use entity::furry;
pub use entity::furry::Entity as Furry;
pub use entity::furry::Column::*;

use rocket::request::{self, Outcome, Request, FromRequest};
use rocket::tokio::time::{sleep, Duration};
use rocket_dyn_templates::{Template, context};

use argon2::{
    password_hash::{
        rand_core::OsRng,
        PasswordHash, PasswordHasher, PasswordVerifier, SaltString
    },
    Argon2
};

struct Hasher<'a>(Argon2<'a>);

use fluent_templates::{FluentLoader, static_loader};

fluent_templates::static_loader! {
    // Declare our `StaticLoader` named `LOCALES`.
    static LOCALES = {
        // The directory of localisations and fluent resources.
        locales: "./locales",
        // The language to falback on if something is not present.
        fallback_language: "en",
        // Optional: A fluent resource that is shared with every locale.
        //core_locales: "./tests/locales/core.ftl",
    };
}

enum Language{
    English,
    French,
    Dutch,
    Furry
}

impl Language{
    fn get(&self)->&'static str {
        match self{
            Language::English => "en",
            Language::French => "fr",
            Language::Dutch => "nl",
            Language::Furry => "fu"
        }
    }
}

impl TryFrom<&str> for Language{
    type Error = ();
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let ci = value.to_ascii_lowercase();
        if ci.starts_with("en"){
            Ok(Self::English)
        }else if ci.starts_with("fr"){
            Ok(Self::French)
        }else if ci.starts_with("nl"){
            Ok(Self::Dutch)
        }else if ci.starts_with("fu"){
            Ok(Self::Furry)
        }else{
            Result::Err(())
        }
    }
}


#[rocket::async_trait]
impl<'r> FromRequest<'r> for Language {
    type Error = std::convert::Infallible;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        if let Some(Ok(val)) = req.query_value::<&str>("lang"){
            if let Ok(lang) = Language::try_from(val){
                return Outcome::Success(lang)
            }
        }else{
            if let Some(s) = req.headers().get_one("Accept-Language"){
                if let Some(Ok(lang)) = s.split(',').map(str::trim_start).map(Language::try_from).find(|v|v.is_ok()){
                    return Outcome::Success(lang)
                }
            }
        }
        Outcome::Success(Language::English)
    }
}

/* 
#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}
*/



#[get("/loutre")]
fn loutre() -> Template {
    Template::render("body", context! { body: "Hello, loutre! 🦦", lang: "fr" })
}


#[get("/delay/<seconds>")]
async fn delay(seconds: u64) -> String {
    sleep(Duration::from_secs(seconds)).await;
    format!("Waited for {} seconds", seconds)
}

#[get("/hello/<name>")]
fn hello(name: &str) -> String {
    format!("Hello, {}!", name)
}

#[get("/?hello&cat=♥")]
fn cats() -> Template {
    Template::render("index", context! { user: "kittens" })
}


#[get("/")]
fn index(lang:Language) -> Template {
    Template::render("index", context! { user: "value", lang: lang.get() })
}

#[get("/add/<name>/<species>")]
async fn add(conn: Connection<'_, Db>, name: String, species:String)->Template{
    let ret = furry::ActiveModel{
        name: Set(name.to_owned()),
        species: Set(Some(species.to_owned())),
        ..Default::default()
    }.save(conn.into_inner()).await.map(|x|format!("{:?}",x));
    Template::render("body", context! { body: ret.unwrap_or("not found".to_owned()), lang: "fr" })
}

#[get("/add")]
async fn add_form(lang:Language,)->Template{
    Template::render("new_fur", context! { lang: lang.get() })
}

#[get("/edit/<id>")]
async fn edit_form(conn: Connection<'_, Db>, lang:Language,id:i32)->Template{
    let ret = 
    Furry::find_by_id(id).one(conn.into_inner()).await.unwrap();

    if let Some(val) = ret{
        Template::render("edit_fur", context! {id, fur:val, lang: lang.get() })

    }else{
        Template::render("body", context! {body: "failed", lang: lang.get() })
    }

}


#[derive(FromForm)]
struct AddFurry{
    name:String,
    species:String,
    password:String,
}
#[derive(FromForm)]
struct EditFurry{
    name:String,
    species:String,
}

#[derive(FromForm)]
struct EditPassword{
    old_password:String,
    password:String,
}

#[derive(FromForm)]
struct Login{
    name:String,
    password:String
}

#[post("/add",data = "<fur>")]
async fn add_post<'a>(h:&State<Hasher<'a>>,conn: Connection<'_, Db>, lang:Language,fur:Form<AddFurry>)->Template{
    let salt = SaltString::generate(&mut OsRng);
    let ret = furry::ActiveModel{
        name: Set(fur.name.to_owned()),
        species: Set(Some(fur.species.to_owned())),
        password_salt:Set(salt.to_string()),
        password_hash:Set(h.inner().0.hash_password(fur.password.as_bytes(), &salt).expect("Hashing failed").to_string()),
        ..Default::default()
    }.save(conn.into_inner()).await.map(|x|format!("{:?}",x));
    Template::render("body", context! { body: ret.unwrap_or("not found".to_owned()), lang: lang.get()})
}

#[get("/login")]
async fn login_form(lang:Language,)->Template{
    Template::render("login", context! { lang: lang.get() })
}



#[post("/login",data="<login>")]
async fn login_post<'a,'b>(cookies:&CookieJar<'b>,h:&State<Hasher<'a>>,conn:Connection<'_, Db>,lang:Language,login:Form<Login>)->Template
{
    let user = &login.name;
    let password = &login.password;
    let user = Furry::find().filter(furry::Column::Name.eq(user.to_owned())).one(conn.into_inner()).await.unwrap();
    if let Some(u) = user{
        if let Ok(hash) = PasswordHash::new(&u.password_hash){
            if h.0.verify_password(password.as_bytes(), &hash).is_ok(){
                cookies.add(Cookie::new("key", "value"));
                Template::render("body", context! { body: "password good", lang: lang.get()})
            }else{
                Template::render("body", context! { body: "password bad", lang: lang.get()})
            }
        }else{
            Template::render("body", context! { body: "something broken", lang: lang.get()})
        }
    }else{
        Template::render("body", context! { body: "user not found", lang: lang.get()})
    }
}

#[post("/edit/<id>",data="<fur>")]
async fn edit_post(conn: Connection<'_, Db>, id:i32, fur:Form<AddFurry>)->Template{
    let ret = furry::ActiveModel{
        name: Set(fur.name.to_owned()),
        species: Set(Some(fur.species.to_owned())),
        id:Unchanged(id),
        ..Default::default()
    }.save(conn.into_inner()).await.map(|x|format!("{:?}",x));
    Template::render("body", context! { body: ret.unwrap_or("not found".to_owned()), lang: "fr" })
}

#[get("/read/<id>")]
async fn read(lang:Language,conn: Connection<'_, Db>, id: i32) -> Template {
    let ret = 
        Furry::find_by_id(id).one(conn.into_inner()).await.unwrap();

    if let Some(val) = ret{
        Template::render("one_fur", context!{fur: val, lang: lang.get() })
    }else{
        Template::render("body", context! {body: "failed", lang: lang.get() })
    }
}



#[get("/list")]
async fn list(lang:Language,conn: Connection<'_, Db>) -> Template {
    let ret = 
        Furry::find().all(conn.into_inner()).await.unwrap();
        //Furry::find_by_id(id).one(conn.into_inner()).await.unwrap();

    if ret.is_empty(){
        Template::render("body", context! {body: "none", lang: lang.get() })
    }else{
        let len = ret.len();
        Template::render("fur_list", context!{furs: ret, len , lang: lang.get() })
    }
}

async fn run_migrations(rocket: Rocket<Build>) -> fairing::Result {
    let conn = &Db::fetch(&rocket).unwrap().conn;
    let _ = migration::Migrator::up(conn, None).await;
    Ok(rocket)
}

lazy_static! {
    static ref PEPPER: String = dotenvy::var("PEPPER").expect(
        "environement variable PEPPER not set"
    );
}

#[launch]
fn rocket() ->  _ {
    let hasher = Hasher(Argon2::new_with_secret(PEPPER.as_bytes(), argon2::Algorithm::default(), argon2::Version::default(), argon2::Params::default()).expect("could not create hasher"));

    rocket::build()
    .attach(Db::init())
    .attach(AdHoc::try_on_ignite("Migrations", run_migrations))
        .attach(Template::custom(|engines|{
            engines.handlebars.register_helper("fluent", Box::new(FluentLoader::new(&*LOCALES)))
        }))
        .manage(hasher)
        .mount("/", routes![index,loutre,delay,hello,cats,read,add,list,add_form,add_post,edit_form,edit_post,login_form,login_post])
}