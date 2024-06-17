use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m_20240517_000003_create_zmessages_table"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let result = manager
            .create_table(
                Table::create()
                    .table(ZMessages::Table)
                    .col(
                        ColumnDef::new(ZMessages::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(ZMessages::MessageId).char_len(64).not_null())
                    .col(ColumnDef::new(ZMessages::Version).unsigned())
                    .col(ColumnDef::new(ZMessages::Type).unsigned().not_null())
                    .col(ColumnDef::new(ZMessages::PublicKey).char_len(64))
                    .col(ColumnDef::new(ZMessages::Data).binary().not_null())
                    .col(ColumnDef::new(ZMessages::Signature).binary())
                    .col(ColumnDef::new(ZMessages::From).char_len(64).not_null())
                    .col(ColumnDef::new(ZMessages::To).char_len(64).not_null())
                    .to_owned(),
            )
            .await;

        if let Err(err) = result {
            return Err(err);
        }

        // create index
        let msgid_index = Index::create()
            .if_not_exists()
            .unique()
            .name("idx-zmessages-messageid")
            .table(ZMessages::Table)
            .col(ZMessages::MessageId)
            .to_owned();
        manager.create_index(msgid_index).await

    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ZMessages::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum ZMessages {
    Table,
    Id,
    MessageId,
    Version,
    Type,
    PublicKey,
    Data,
    Signature,
    From,
    To,
}