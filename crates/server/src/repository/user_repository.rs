use crate::entity::user;
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use jisi_code_core::domain::UserId;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct UserRecord {
    pub id: UserId,
    pub username: String,
    pub email: String,
}

#[derive(Debug, Clone)]
pub struct NewUser {
    pub username: String,
    pub email: String,
}

#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn create(&self, new_user: NewUser) -> Result<UserRecord>;
    async fn find_by_id(&self, user_id: UserId) -> Result<Option<UserRecord>>;
    async fn find_by_username(&self, username: &str) -> Result<Option<UserRecord>>;
}

#[derive(Clone)]
pub struct SeaOrmUserRepository {
    db: DatabaseConnection,
}

impl SeaOrmUserRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    fn map_model(model: user::Model) -> Result<UserRecord> {
        let id = UserId::from_str(&model.id)
            .map_err(|e| anyhow!("invalid user.id '{}' from database: {e}", model.id))?;

        Ok(UserRecord {
            id,
            username: model.username,
            email: model.email,
        })
    }
}

#[async_trait]
impl UserRepository for SeaOrmUserRepository {
    async fn create(&self, new_user: NewUser) -> Result<UserRecord> {
        let id = UserId::new();

        let active_model = user::ActiveModel {
            id: Set(id.to_string()),
            username: Set(new_user.username),
            email: Set(new_user.email),
            ..Default::default()
        };

        let model = active_model.insert(&self.db).await?;
        Self::map_model(model)
    }

    async fn find_by_id(&self, user_id: UserId) -> Result<Option<UserRecord>> {
        let model = user::Entity::find_by_id(user_id.to_string())
            .one(&self.db)
            .await?;

        model.map(Self::map_model).transpose()
    }

    async fn find_by_username(&self, username: &str) -> Result<Option<UserRecord>> {
        let model = user::Entity::find()
            .filter(user::Column::Username.eq(username))
            .one(&self.db)
            .await?;

        model.map(Self::map_model).transpose()
    }
}
