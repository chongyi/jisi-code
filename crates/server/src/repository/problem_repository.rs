use crate::entity::problem;
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use jisi_code_core::domain::{Difficulty, ProblemId};
use sea_orm::{ActiveModelTrait, ActiveValue::Set, DatabaseConnection, EntityTrait};
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct ProblemRecord {
    pub id: ProblemId,
    pub title: String,
    pub description: String,
    pub difficulty: Difficulty,
}

#[derive(Debug, Clone)]
pub struct NewProblem {
    pub title: String,
    pub description: String,
    pub difficulty: Difficulty,
}

#[async_trait]
pub trait ProblemRepository: Send + Sync {
    async fn create(&self, new_problem: NewProblem) -> Result<ProblemRecord>;
    async fn find_by_id(&self, problem_id: ProblemId) -> Result<Option<ProblemRecord>>;
}

#[derive(Clone)]
pub struct SeaOrmProblemRepository {
    db: DatabaseConnection,
}

impl SeaOrmProblemRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    fn map_difficulty(code: i16) -> Result<Difficulty> {
        match code {
            0 => Ok(Difficulty::Easy),
            1 => Ok(Difficulty::Medium),
            2 => Ok(Difficulty::Hard),
            _ => Err(anyhow!("invalid problem.difficulty code from database: {code}")),
        }
    }

    fn map_difficulty_code(difficulty: Difficulty) -> i16 {
        match difficulty {
            Difficulty::Easy => 0,
            Difficulty::Medium => 1,
            Difficulty::Hard => 2,
        }
    }

    fn map_model(model: problem::Model) -> Result<ProblemRecord> {
        let id = ProblemId::from_str(&model.id)
            .map_err(|e| anyhow!("invalid problem.id '{}' from database: {e}", model.id))?;

        Ok(ProblemRecord {
            id,
            title: model.title,
            description: model.description,
            difficulty: Self::map_difficulty(model.difficulty)?,
        })
    }
}

#[async_trait]
impl ProblemRepository for SeaOrmProblemRepository {
    async fn create(&self, new_problem: NewProblem) -> Result<ProblemRecord> {
        let id = ProblemId::new();

        let active_model = problem::ActiveModel {
            id: Set(id.to_string()),
            title: Set(new_problem.title),
            description: Set(new_problem.description),
            difficulty: Set(Self::map_difficulty_code(new_problem.difficulty)),
            ..Default::default()
        };

        let model = active_model.insert(&self.db).await?;
        Self::map_model(model)
    }

    async fn find_by_id(&self, problem_id: ProblemId) -> Result<Option<ProblemRecord>> {
        let model = problem::Entity::find_by_id(problem_id.to_string())
            .one(&self.db)
            .await?;

        model.map(Self::map_model).transpose()
    }
}
