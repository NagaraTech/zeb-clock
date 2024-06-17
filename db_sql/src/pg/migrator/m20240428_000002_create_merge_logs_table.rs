use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m_20240428_000001_create_merge_logs_table"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    // Define how to apply this migration: Create the Mergelogs table.
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let result = manager
            .create_table(
                Table::create()
                    .table(MergeLogs::Table)
                    .col(
                        ColumnDef::new(MergeLogs::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(MergeLogs::FromId).char_len(64).not_null())
                    .col(ColumnDef::new(MergeLogs::ToId).char_len(64).not_null())
                    .col(ColumnDef::new(MergeLogs::StartCount).big_unsigned().not_null())
                    .col(ColumnDef::new(MergeLogs::EndCount).big_unsigned().not_null())
                    .col(ColumnDef::new(MergeLogs::SClockHash).char_len(64).not_null())
                    .col(ColumnDef::new(MergeLogs::EClockHash).char_len(64).not_null())
                    .col(ColumnDef::new(MergeLogs::MergeAt).timestamp().not_null())
                    .to_owned(),
            )
            .await;
        
        if let Err(err) = result {
            return Err(err);
        }    
        // create index
        let msgid_index = Index::create()
            .if_not_exists()
            .name("idx-mergelogs-sclockhash")
            .table(MergeLogs::Table)
            .col(MergeLogs::SClockHash)
            .to_owned();
        let result = manager.create_index(msgid_index).await; 
        if let Err(err) = result {
            return Err(err);
        }

        let nodeid_index = Index::create()
            .if_not_exists()
            .name("idx-mergelogs-eclockhash")
            .table(MergeLogs::Table)
            .col(MergeLogs::EClockHash)
            .to_owned();
        manager.create_index(nodeid_index).await
    }

    // Define how to rollback this migration: Drop the Chef table.
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(MergeLogs::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum MergeLogs {
    Table,
    Id,
    FromId,
    ToId,
    StartCount,
    EndCount,
    SClockHash,
    EClockHash,
    MergeAt
}