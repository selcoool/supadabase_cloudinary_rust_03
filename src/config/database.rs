use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

pub async fn create_pool() -> Result<PgPool, sqlx::Error> {
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL is missing from .env");

    PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
}