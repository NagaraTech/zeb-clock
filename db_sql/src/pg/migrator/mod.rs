use sea_orm_migration::prelude::*;

mod m20240428_000001_create_clock_infos_table;
mod m20240428_000002_create_merge_logs_table;
mod m20240517_000003_create_zmessages_table;
mod m20240529_000004_create_business_clocks_table;

/// Use the sea-orm-cli to generate data entity, 
/// command like as follow:
/// 
/// ```rust
/// sea-orm-cli generate entity \
/// -u mysql://root:password@localhost:3306/bakeries_db \
/// -o src/entities
/// ```
/// 
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20240428_000001_create_clock_infos_table::Migration),
            Box::new(m20240428_000002_create_merge_logs_table::Migration),
            Box::new(m20240517_000003_create_zmessages_table::Migration),
            Box::new(m20240529_000004_create_business_clocks_table::Migration),
        ]
    }
}