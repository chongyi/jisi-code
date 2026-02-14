use sea_orm_migration::prelude::*;
use sea_orm_migration::schema::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(User::Table)
                    .if_not_exists()
                    .col(string_len(User::Id, 36).primary_key())
                    .col(string_len(User::Username, 50).unique_key())
                    .col(string_len(User::Email, 255).unique_key())
                    .col(timestamp(User::CreatedAt).default(Expr::current_timestamp()))
                    .col(timestamp(User::UpdatedAt).default(Expr::current_timestamp()))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Problem::Table)
                    .if_not_exists()
                    .col(string_len(Problem::Id, 36).primary_key())
                    .col(string_len(Problem::Title, 200))
                    .col(text(Problem::Description))
                    // Difficulty enum is represented in app code. DB stores compact numeric code.
                    // 0=easy, 1=medium, 2=hard
                    .col(
                        small_integer(Problem::Difficulty)
                            .check(Expr::col(Problem::Difficulty).gte(0))
                            .check(Expr::col(Problem::Difficulty).lte(2)),
                    )
                    .col(timestamp(Problem::CreatedAt).default(Expr::current_timestamp()))
                    .col(timestamp(Problem::UpdatedAt).default(Expr::current_timestamp()))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Submission::Table)
                    .if_not_exists()
                    .col(string_len(Submission::Id, 36).primary_key())
                    .col(string_len(Submission::UserId, 36))
                    .col(string_len(Submission::ProblemId, 36))
                    // Language enum is represented in app code.
                    // 0=rust, 1=cpp, 2=java, 3=python, 4=go, 5=javascript, 6=typescript
                    .col(
                        small_integer(Submission::Language)
                            .check(Expr::col(Submission::Language).gte(0))
                            .check(Expr::col(Submission::Language).lte(6)),
                    )
                    // SubmissionStatus enum is represented in app code.
                    // 0=pending, 1=running, 2=accepted, 3=wrong_answer,
                    // 4=time_limit_exceeded, 5=runtime_error, 6=compile_error, 7=internal_error
                    .col(
                        small_integer(Submission::Status)
                            .check(Expr::col(Submission::Status).gte(0))
                            .check(Expr::col(Submission::Status).lte(7)),
                    )
                    .col(
                        small_integer(Submission::Score)
                            .default(0)
                            .check(Expr::col(Submission::Score).gte(0))
                            .check(Expr::col(Submission::Score).lte(100)),
                    )
                    .col(text(Submission::SourceCode))
                    .col(text_null(Submission::CompilerOutput))
                    .col(text_null(Submission::ExecutionOutput))
                    .col(integer_null(Submission::RuntimeMs))
                    .col(integer_null(Submission::MemoryKb))
                    .col(timestamp(Submission::CreatedAt).default(Expr::current_timestamp()))
                    .col(timestamp(Submission::UpdatedAt).default(Expr::current_timestamp()))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-submissions-user_id")
                            .from(Submission::Table, Submission::UserId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-submissions-problem_id")
                            .from(Submission::Table, Submission::ProblemId)
                            .to(Problem::Table, Problem::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_submissions_user_id")
                    .table(Submission::Table)
                    .col(Submission::UserId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_submissions_problem_id")
                    .table(Submission::Table)
                    .col(Submission::ProblemId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_submissions_status")
                    .table(Submission::Table)
                    .col(Submission::Status)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_submissions_created_at")
                    .table(Submission::Table)
                    .col(Submission::CreatedAt)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Submission::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(Problem::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(User::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum User {
    Table,
    Id,
    Username,
    Email,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Problem {
    Table,
    Id,
    Title,
    Description,
    Difficulty,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Submission {
    Table,
    Id,
    UserId,
    ProblemId,
    Language,
    Status,
    Score,
    SourceCode,
    CompilerOutput,
    ExecutionOutput,
    RuntimeMs,
    MemoryKb,
    CreatedAt,
    UpdatedAt,
}
