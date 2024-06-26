//! `SeaORM` Entity. Generated by sea-orm-codegen 0.12.15

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "bussiness_clocks")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub category: String,
    pub address: String,
    pub clock: Json,
    pub clock_hash: String,
    pub tag: String,
    pub event_count: i64,
    pub message_id: String,
    pub create_at: Option<DateTime>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
