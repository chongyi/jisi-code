use crate::entity::submission;
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use jisi_code_core::domain::{Language, ProblemId, Score, SubmissionId, SubmissionStatus, UserId};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
};
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct SubmissionRecord {
    pub id: SubmissionId,
    pub user_id: UserId,
    pub problem_id: ProblemId,
    pub language: Language,
    pub status: SubmissionStatus,
    pub score: Score,
    pub source_code: String,
    pub compiler_output: Option<String>,
    pub execution_output: Option<String>,
    pub runtime_ms: Option<i32>,
    pub memory_kb: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct NewSubmission {
    pub user_id: UserId,
    pub problem_id: ProblemId,
    pub language: Language,
    pub source_code: String,
}

#[derive(Debug, Clone)]
pub struct UpdateSubmissionResult {
    pub status: SubmissionStatus,
    pub score: Score,
    pub compiler_output: Option<String>,
    pub execution_output: Option<String>,
    pub runtime_ms: Option<i32>,
    pub memory_kb: Option<i32>,
}

#[async_trait]
pub trait SubmissionRepository: Send + Sync {
    async fn create(&self, new_submission: NewSubmission) -> Result<SubmissionRecord>;
    async fn find_by_id(&self, submission_id: SubmissionId) -> Result<Option<SubmissionRecord>>;
    async fn list_by_user_id(&self, user_id: UserId) -> Result<Vec<SubmissionRecord>>;
    async fn update_result(
        &self,
        submission_id: SubmissionId,
        update: UpdateSubmissionResult,
    ) -> Result<Option<SubmissionRecord>>;
}

#[derive(Clone)]
pub struct SeaOrmSubmissionRepository {
    db: DatabaseConnection,
}

impl SeaOrmSubmissionRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    fn map_language(code: i16) -> Result<Language> {
        match code {
            0 => Ok(Language::Rust),
            1 => Ok(Language::Cpp),
            2 => Ok(Language::Java),
            3 => Ok(Language::Python),
            4 => Ok(Language::Go),
            5 => Ok(Language::JavaScript),
            6 => Ok(Language::TypeScript),
            _ => Err(anyhow!("invalid submission.language code from database: {code}")),
        }
    }

    fn map_language_code(language: Language) -> i16 {
        match language {
            Language::Rust => 0,
            Language::Cpp => 1,
            Language::Java => 2,
            Language::Python => 3,
            Language::Go => 4,
            Language::JavaScript => 5,
            Language::TypeScript => 6,
        }
    }

    fn map_status(code: i16) -> Result<SubmissionStatus> {
        match code {
            0 => Ok(SubmissionStatus::Pending),
            1 => Ok(SubmissionStatus::Running),
            2 => Ok(SubmissionStatus::Accepted),
            3 => Ok(SubmissionStatus::WrongAnswer),
            4 => Ok(SubmissionStatus::TimeLimitExceeded),
            5 => Ok(SubmissionStatus::RuntimeError),
            6 => Ok(SubmissionStatus::CompileError),
            7 => Ok(SubmissionStatus::InternalError),
            _ => Err(anyhow!("invalid submission.status code from database: {code}")),
        }
    }

    fn map_status_code(status: SubmissionStatus) -> i16 {
        match status {
            SubmissionStatus::Pending => 0,
            SubmissionStatus::Running => 1,
            SubmissionStatus::Accepted => 2,
            SubmissionStatus::WrongAnswer => 3,
            SubmissionStatus::TimeLimitExceeded => 4,
            SubmissionStatus::RuntimeError => 5,
            SubmissionStatus::CompileError => 6,
            SubmissionStatus::InternalError => 7,
        }
    }

    fn map_model(model: submission::Model) -> Result<SubmissionRecord> {
        let id = SubmissionId::from_str(&model.id)
            .map_err(|e| anyhow!("invalid submission.id '{}' from database: {e}", model.id))?;
        let user_id = UserId::from_str(&model.user_id).map_err(|e| {
            anyhow!(
                "invalid submission.user_id '{}' from database: {e}",
                model.user_id
            )
        })?;
        let problem_id = ProblemId::from_str(&model.problem_id).map_err(|e| {
            anyhow!(
                "invalid submission.problem_id '{}' from database: {e}",
                model.problem_id
            )
        })?;

        let score_u16 = u16::try_from(model.score).map_err(|_| {
            anyhow!(
                "invalid submission.score from database: {} (must be non-negative)",
                model.score
            )
        })?;

        Ok(SubmissionRecord {
            id,
            user_id,
            problem_id,
            language: Self::map_language(model.language)?,
            status: Self::map_status(model.status)?,
            score: Score::new(score_u16)?,
            source_code: model.source_code,
            compiler_output: model.compiler_output,
            execution_output: model.execution_output,
            runtime_ms: model.runtime_ms,
            memory_kb: model.memory_kb,
        })
    }
}

#[async_trait]
impl SubmissionRepository for SeaOrmSubmissionRepository {
    async fn create(&self, new_submission: NewSubmission) -> Result<SubmissionRecord> {
        let id = SubmissionId::new();

        let active_model = submission::ActiveModel {
            id: Set(id.to_string()),
            user_id: Set(new_submission.user_id.to_string()),
            problem_id: Set(new_submission.problem_id.to_string()),
            language: Set(Self::map_language_code(new_submission.language)),
            status: Set(Self::map_status_code(SubmissionStatus::Pending)),
            score: Set(i16::try_from(u16::from(Score::default()))?),
            source_code: Set(new_submission.source_code),
            compiler_output: Set(None),
            execution_output: Set(None),
            runtime_ms: Set(None),
            memory_kb: Set(None),
            ..Default::default()
        };

        let model = active_model.insert(&self.db).await?;
        Self::map_model(model)
    }

    async fn find_by_id(&self, submission_id: SubmissionId) -> Result<Option<SubmissionRecord>> {
        let model = submission::Entity::find_by_id(submission_id.to_string())
            .one(&self.db)
            .await?;

        model.map(Self::map_model).transpose()
    }

    async fn list_by_user_id(&self, user_id: UserId) -> Result<Vec<SubmissionRecord>> {
        let models = submission::Entity::find()
            .filter(submission::Column::UserId.eq(user_id.to_string()))
            .all(&self.db)
            .await?;

        models.into_iter().map(Self::map_model).collect()
    }

    async fn update_result(
        &self,
        submission_id: SubmissionId,
        update: UpdateSubmissionResult,
    ) -> Result<Option<SubmissionRecord>> {
        let Some(model) = submission::Entity::find_by_id(submission_id.to_string())
            .one(&self.db)
            .await?
        else {
            return Ok(None);
        };

        let mut active_model: submission::ActiveModel = model.into();
        active_model.status = Set(Self::map_status_code(update.status));
        active_model.score = Set(i16::try_from(u16::from(update.score))?);
        active_model.compiler_output = Set(update.compiler_output);
        active_model.execution_output = Set(update.execution_output);
        active_model.runtime_ms = Set(update.runtime_ms);
        active_model.memory_kb = Set(update.memory_kb);

        let updated = active_model.update(&self.db).await?;
        Self::map_model(updated).map(Some)
    }
}
