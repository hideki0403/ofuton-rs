use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Object::Table)
                    .add_column(ColumnDef::new(Object::EncodedFilename).string().null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(Table::alter().table(Object::Table).drop_column(Object::EncodedFilename).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Object {
    Table,
    EncodedFilename,
}
