pub use sea_orm_migration::prelude::*;

mod m20221117_104400_create_furry_table;
mod m20221122_181200_add_password_to_furry_table;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(m20221117_104400_create_furry_table::Migration),
             Box::new(m20221122_181200_add_password_to_furry_table::Migration)]
    }
}
