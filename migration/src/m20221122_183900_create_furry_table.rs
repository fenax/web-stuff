use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Furry::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Furry::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Furry::Name).string().not_null())
                    .col(ColumnDef::new(Furry::Species).string())
                    .col(ColumnDef::new(Furry::PasswordHash).string().not_null())
                    .col(ColumnDef::new(Furry::PasswordSalt).string().not_null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Furry::Table).to_owned())
            .await
    }
}

/// Learn more at https://docs.rs/sea-query#iden
#[derive(Iden)]
enum Furry {
    Table,
    Id,
    Name,
    Species,
    PasswordHash,
    PasswordSalt,
}
