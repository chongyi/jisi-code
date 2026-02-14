use jisi_code_migration::{Migrator, MigratorTrait};
use sea_orm::{Database, DatabaseConnection};
use std::env;

pub async fn init_pool_and_migrate() -> anyhow::Result<DatabaseConnection> {
    let database_url =
        env::var("DATABASE_URL").map_err(|_| anyhow::anyhow!("DATABASE_URL is not set"))?;

    let db = Database::connect(&database_url).await?;

    Migrator::up(&db, None).await?;

    Ok(db)
}
