use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.create_table(
            Table::create()
                .table(Object::Table)
                .if_not_exists()
                .col(pk_auto(Object::Id))
                .col(ColumnDef::new(Object::Path).string().not_null().unique_key())
                .col(ColumnDef::new(Object::Filename).string().not_null())
                .col(ColumnDef::new(Object::ContentSize).big_unsigned().not_null())
                .col(ColumnDef::new(Object::MimeType).string().not_null())
                .primary_key(Index::create().name("idx_objects_id").col(Object::Id))
                .index(Index::create().name("idx_objects_path").col(Object::Path).unique())
                .to_owned(),
        ).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(Object::Table).to_owned()).await
    }
}

#[derive(DeriveIden)]
enum Object {
    Table,
    Id,
    Path,
    Filename,
    ContentSize,
    MimeType,
}
