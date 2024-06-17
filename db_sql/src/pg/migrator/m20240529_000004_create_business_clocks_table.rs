use sea_orm_migration::prelude::*;
use sea_query::Index;
pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m_20240529_000004_create_business_clocks_table.rs"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    // Define how to apply this migration: Create the clock_infos table.
    // or use manager.alter_table(stmt) to update the table for migration.
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let result = manager
            .create_table(
                Table::create()
                    .table(BussinessClocks::Table)
                    .col(
                        ColumnDef::new(BussinessClocks::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(BussinessClocks::Category).char_len(64).not_null())
                    .col(ColumnDef::new(BussinessClocks::Address).string().not_null())
                    .col(ColumnDef::new(BussinessClocks::Clock).json().not_null())
                    .col(ColumnDef::new(BussinessClocks::ClockHash).char_len(64).not_null())
                    .col(ColumnDef::new(BussinessClocks::Tag).char_len(64).not_null())
                    .col(ColumnDef::new(BussinessClocks::EventCount).big_unsigned().not_null())
                    .col(ColumnDef::new(BussinessClocks::MessageId).char_len(64).not_null())
                    .col(ColumnDef::new(BussinessClocks::CreateAt).timestamp())
                    .to_owned(),
            ).await;

        if let Err(err) = result {
            return Err(err);
        }    
        
        // create index
        let msgid_index = Index::create()
            .if_not_exists()
            .name("idx-businessclocks-messageid")
            .table(BussinessClocks::Table)
            .col(BussinessClocks::MessageId)
            .to_owned();
        let result = manager.create_index(msgid_index).await; 
        if let Err(err) = result {
            return Err(err);
        }

        let category_index = Index::create()
            .if_not_exists()
            .name("idx-businessclocks-category")
            .table(BussinessClocks::Table)
            .col(BussinessClocks::Category)
            .to_owned();
        let result = manager.create_index(category_index).await; 
        if let Err(err) = result {
            return Err(err);
        }

        let tag_index = Index::create()
            .if_not_exists()
            .name("idx-businessclocks-tag")
            .table(BussinessClocks::Table)
            .col(BussinessClocks::Tag)
            .to_owned();
        let result = manager.create_index(tag_index).await; 
        if let Err(err) = result {
            return Err(err);
        }

        let address_index = Index::create()
            .if_not_exists()
            .name("idx-businessclocks-addressd")
            .table(BussinessClocks::Table)
            .col(BussinessClocks::Address)
            .to_owned();
        manager.create_index(address_index).await
    }

    // Define how to rollback this migration: Drop the ClockInfo table.
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(BussinessClocks::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum BussinessClocks {
    Table,
    Id,
    Category,    // project events or task workflow hash
    Address,     // nodeid or asset address
    Clock,
    ClockHash,
    Tag,         // hash for describing which stage about clock when causality dependence
    EventCount,
    MessageId,
    CreateAt
}