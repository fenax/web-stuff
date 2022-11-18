#[macro_use]
extern crate rocket;


use rocket::fairing::{self, AdHoc};
use rocket::form::{Context, Form};
use rocket::fs::{relative, FileServer};
use rocket::response::{Flash, Redirect};
use rocket::{Build, Rocket};

use migration::{MigratorTrait, DbErr};
use sea_orm::{EntityTrait, ActiveModelTrait, Set};
use sea_orm_rocket::{Connection, Database};

mod pool;
use pool::Db;

pub use entity::furry;
pub use entity::furry::Entity as Furry;

use rocket::http::Status;
use rocket::request::{self, Outcome, FlashMessage, Request, FromRequest};
use rocket::tokio::time::{sleep, Duration};
use rocket_dyn_templates::{Template, context};
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

#[get("/read/<id>")]
async fn read(conn: Connection<'_, Db>, id: i32) -> Template {
    let ret = 
        Furry::find_by_id(id).one(conn.into_inner()).await.unwrap().map(|x|format!("{:?}",x));

        Template::render("body", context! { body: ret.unwrap_or("not found".to_owned()), lang: "fr" })

}

async fn run_migrations(rocket: Rocket<Build>) -> fairing::Result {
    let conn = &Db::fetch(&rocket).unwrap().conn;
    let _ = migration::Migrator::up(conn, None).await;
    Ok(rocket)
}

#[launch]
fn rocket() ->  _ {
    rocket::build()
    .attach(Db::init())
    .attach(AdHoc::try_on_ignite("Migrations", run_migrations))
        .attach(Template::custom(|engines|{
            engines.handlebars.register_helper("fluent", Box::new(FluentLoader::new(&*LOCALES)))
        }))
        .mount("/", routes![index,loutre,delay,hello,cats,read,add])
}