use crate::{AggregateRoot, Presentation};

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, DeriveEntityModel)]
#[sea_orm(table_name = "users")]
pub struct Model {
  #[sea_orm(primary_key, auto_increment = false)]
  pub id: Uuid,
  pub name: String,
  pub role: Role,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl AggregateRoot<'_> for Model {
  fn id(&self) -> Uuid {
    self.id
  }
}

impl Presentation<'_> for Model {}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "String(Some(1))")]
pub enum Role {
  #[sea_orm(string_value = "U")]
  User,
  #[sea_orm(string_value = "A")]
  Admin,
}
