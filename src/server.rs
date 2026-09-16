use dioxus::prelude::*;

#[cfg(feature = "server")]
use std::sync::OnceLock;

#[cfg(feature = "server")]
pub static DB_POOL: OnceLock<sqlx::PgPool> = OnceLock::new();

#[cfg(feature = "server")]
use dioxus::server::axum::{
    body::Body,
    response::Response,
};

#[cfg(feature = "server")]
pub async fn create_db_pool() -> Result<(), ServerFnError> {
    dotenvy::dotenv().ok();

    let database_url =
        std::env::var("DATABASE_URL")
            .map_err(|e| ServerFnError::new(e.to_string()))?;

    let pool = sqlx::PgPool::connect(&database_url)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    DB_POOL
        .set(pool)
        .map_err(|_| ServerFnError::new("Database pool already initialized"))?;

    Ok(())
}

#[server]
pub async fn test_database() -> Result<i32, ServerFnError> {
    let pool = DB_POOL
        .get()
        .ok_or_else(|| ServerFnError::new("Database pool not initialized"))?;

    let result = sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(result)
}