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
use crate::ilanders;
use crate::ilanders::Ilander;

#[cfg(feature = "server")]
use argon2::{
    Argon2,
    PasswordHash,
    PasswordVerifier,
    PasswordHasher
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

#[server]
pub async fn get_ilanders() -> Result<Vec<Ilander>, ServerFnError> {
    let pool = DB_POOL
        .get()
        .ok_or_else(|| ServerFnError::new("Database not initialized"))?;

    let ilanders = sqlx::query_as::<_, ilanders::Ilander>(
        "SELECT * FROM ilander ORDER BY name"
    )
        .fetch_all(pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(ilanders)
}

#[server]
pub async fn login(
    username: String,
    password: String,
) -> Result<(), ServerFnError> {
    let pool = DB_POOL
        .get()
        .ok_or_else(|| ServerFnError::new("Database pool not initialized"))?;

    let password_hash: String = sqlx::query_scalar(
        "SELECT password_hash FROM users WHERE username = $1"
    )
        .bind(&username)
        .fetch_optional(pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .ok_or_else(|| ServerFnError::new("Nom d'utilisateur ou mot de passe incorrect"))?;

    Argon2::default()
        .verify_password(password.as_bytes(), password_hash.as_str())
        .map_err(|_| {
            ServerFnError::new("Nom d'utilisateur ou mot de passe incorrect")
        })?;


    Ok(())
}

#[server]
pub async fn register(
    username: String,
    password: String,
) -> Result<(), ServerFnError> {
    let pool = DB_POOL
        .get()
        .ok_or_else(|| ServerFnError::new("Database pool not initialized"))?;

    // Vérifier que le nom n'est pas déjà utilisé
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(
            SELECT 1
            FROM users
            WHERE username = $1
        )"
    )
        .bind(&username)
        .fetch_one(pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    if exists {
        return Err(ServerFnError::new("Nom d'utilisateur déjà utilisé"));
    }

    // Hash du mot de passe
    let password_hash = Argon2::default()
        .hash_password(password.as_bytes())
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .to_string();

    // Insérer l'utilisateur
    sqlx::query(
        "INSERT INTO users (username, password_hash)
         VALUES ($1, $2)"
    )
        .bind(&username)
        .bind(&password_hash)
        .execute(pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(())
}