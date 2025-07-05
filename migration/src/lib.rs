pub use sea_orm_migration::prelude::*;

mod m20250702_134901_create_objects_table;
mod m20250705_083629_add_internal_filename_column_to_object_table;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20250702_134901_create_objects_table::Migration),
            Box::new(m20250705_083629_add_internal_filename_column_to_object_table::Migration),
        ]
    }
}
