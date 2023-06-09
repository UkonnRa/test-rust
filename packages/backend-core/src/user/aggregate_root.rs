use crate::user::{
  ActiveModel, Column, Command, CommandCreate, CommandDelete, CommandUpdate, Entity, Model,
  Presentation, PrimaryKey, Query, Role, User,
};
use crate::{
  AggregateRoot, Error, FindAllArgs, Permission, Repository, Result, FIELD_ID, FIELD_NAME,
};
use futures::TryStreamExt;
use sea_orm::entity::prelude::*;
use sea_orm::StreamTrait;
use std::cmp::Ordering;
use std::collections::HashMap;
use uuid::Uuid;

const FIELD_ROLE: &str = "role";

#[async_trait::async_trait]
impl AggregateRoot for User {
  type Model = Model;
  type ActiveModel = ActiveModel;
  type Entity = Entity;
  type Presentation = Presentation;
  type PrimaryKey = PrimaryKey;
  type Query = Query;
  type Column = Column;
  type Command = Command;

  fn typ() -> &'static str {
    "User"
  }

  fn id(&self) -> Uuid {
    self.id
  }

  fn primary_column() -> Column {
    Column::Id
  }

  fn compare_by_field(&self, other: &Self, field: impl ToString) -> Option<Ordering> {
    match field.to_string().as_str() {
      FIELD_ID => Some(self.id.cmp(&other.id)),
      FIELD_NAME => Some(self.name.cmp(&other.name)),
      FIELD_ROLE => Some(self.role.cmp(&other.role)),
      _ => None,
    }
  }

  async fn from_models(_db: &impl ConnectionTrait, models: Vec<Self::Model>) -> Result<Vec<Self>> {
    Ok(models)
  }

  async fn handle(
    db: &(impl ConnectionTrait + StreamTrait),
    operator: Option<&Model>,
    command: Self::Command,
  ) -> Result<Vec<Self>> {
    Ok(match command {
      Command::Create(command) => vec![Model::create(db, operator, command).await?],
      Command::Update(command) => vec![Model::update(db, operator, command).await?],
      Command::Delete(command) => {
        let _ = Model::delete(db, operator, command).await?;
        vec![]
      }
    })
  }

  async fn get_permission(
    _db: &impl ConnectionTrait,
    operator: Option<&Self>,
    models: &[Self],
  ) -> Result<HashMap<Uuid, Permission>> {
    Ok(
      models
        .iter()
        .map(|model| {
          (
            model.id(),
            match operator {
              Some(operator) if operator.role == Role::Admin || operator.id == model.id() => {
                Permission::ReadWrite
              }
              _ => Permission::ReadOnly,
            },
          )
        })
        .collect::<HashMap<_, _>>(),
    )
  }
}

impl Model {
  pub fn new(name: impl ToString, role: Role) -> Model {
    Model { id: Uuid::new_v4(), name: name.to_string(), role }
  }

  async fn validate_name(
    db: &(impl ConnectionTrait + StreamTrait),
    name: impl ToString,
  ) -> Result<String> {
    let name = name.to_string().trim().to_string();
    if Repository::<Model>::do_find_all(
      db,
      FindAllArgs {
        query: Query { name: (name.clone(), false), ..Default::default() },
        ..Default::default()
      },
    )
    .await?
    .try_next()
    .await?
    .is_some()
    {
      Err(Error::already_exists::<Self>(vec![(FIELD_NAME, name)]))
    } else if name.len() < 4 || name.len() > 128 {
      Err(Error::NotInRange { field: "name.length".to_string(), begin: 4, end: 128 })
    } else {
      Ok(name)
    }
  }

  fn validate_role(operator: Option<&Model>, role: Role) -> Result<Role> {
    if operator.map(|operator| operator.role >= role).unwrap_or_else(|| role == Role::User) {
      Ok(role)
    } else {
      Err(Error::NoWritePermission {
        operator_id: operator.map(User::id),
        typ: User::typ().to_string(),
        field_values: vec![(FIELD_ROLE.to_string(), role.to_string())],
      })
    }
  }

  async fn create(
    db: &(impl ConnectionTrait + StreamTrait),
    operator: Option<&Model>,
    command: CommandCreate,
  ) -> Result<Model> {
    let name = Self::validate_name(db, command.name).await?;
    let role = Self::validate_role(operator, command.role)?;
    let user = Model::new(name, role);

    Ok(Repository::<Model>::save(db, vec![user]).await?.into_iter().last().unwrap())
  }

  async fn update(
    db: &(impl ConnectionTrait + StreamTrait),
    operator: Option<&Model>,
    command: CommandUpdate,
  ) -> Result<Model> {
    let id: Uuid = command.id.clone().parse()?;
    let mut model = Repository::<User>::find_by_id(db, id)
      .await?
      .ok_or_else(|| Error::not_found::<User>(vec![(FIELD_ID, id)]))?;

    if command.is_empty() {
      return Ok(model);
    }

    Self::check_writeable(db, operator, &[model.clone()]).await?;

    if let Some(name) = command.name {
      model.name = Self::validate_name(db, name).await?;
    }

    if let Some(role) = command.role {
      model.role = Self::validate_role(operator, role)?;
    }

    Ok(model)
  }

  async fn delete(
    db: &(impl ConnectionTrait + StreamTrait),
    operator: Option<&Model>,
    command: CommandDelete,
  ) -> Result<()> {
    if command.id.is_empty() {
      return Ok(());
    }

    let models = Repository::<User>::find_by_ids(db, command.id).await?;
    Self::check_writeable(db, operator, &models).await?;
    Repository::delete(db, models).await
  }
}
