use dioxus::prelude::*;

#[cfg(feature = "server")]
use std::collections::HashMap;

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
use dioxus::fullstack::FullstackContext;
#[cfg(feature = "server")]
use dioxus::server::http::{
    header::COOKIE,
    header::SET_COOKIE,
    HeaderValue,
};

use serde::{Deserialize, Serialize};

#[cfg(feature = "server")]
use sha2::{Digest, Sha256};

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

    let (user_id, password_hash): (i32, String) = sqlx::query_as(
        "SELECT id, password_hash FROM users WHERE username = $1"
    )
        .bind(&username)
        .fetch_optional(pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .ok_or_else(|| ServerFnError::new("Wrong username or password."))?;

    Argon2::default()
        .verify_password(password.as_bytes(), password_hash.as_str())
        .map_err(|_| {
            ServerFnError::new("Wrong username or password.")
        })?;

    create_session(pool, user_id).await?;

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
        return Err(ServerFnError::new("Username is taken."));
    }

    // Hash du mot de passe
    let password_hash = Argon2::default()
        .hash_password(password.as_bytes())
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .to_string();

    // Insérer l'utilisateur
    let user_id: i32 = sqlx::query_scalar::<_, i32>(
        "INSERT INTO users (username, password_hash)
         VALUES ($1, $2)
         RETURNING id"
    )
        .bind(&username)
        .bind(&password_hash)
        .fetch_one(pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    create_session(pool, user_id).await?;

    Ok(())
}

/// Secure par défaut ; désactivation explicite pour le développement HTTP.
#[cfg(feature = "server")]
pub fn secure() -> Result<bool, ServerFnError>{
    match std::env::var("SESSION_COOKIE_SECURE") {
        Ok(value) if value == "false" => Ok(false),
        Ok(value) if value == "true" => Ok(true),
        Err(std::env::VarError::NotPresent) => Ok(true),
        _ => {
            Err(ServerFnError::new(
                "Invalid session cookie configuration.",
            ))
        }
    }
}

#[cfg(feature = "server")]
async fn create_session(
    pool: &sqlx::PgPool,
    user_id: i32,
) -> Result<(), ServerFnError> {
    const SESSION_DURATION_SECONDS: i64 = 7 * 24 * 60 * 60;

    let context = FullstackContext::current()
        .ok_or_else(|| ServerFnError::new("Request context unavailable."))?;

    let secure = secure()?;

    // 32 octets aléatoires = un secret de 256 bits.
    let mut random_bytes = [0_u8; 32];
    getrandom::fill(&mut random_bytes).map_err(|error| {
        eprintln!("Session token generation failed: {error}");
        ServerFnError::new("Unable to create session.")
    })?;

    let token = hex::encode(random_bytes);

    // Le navigateur reçoit le secret ; PostgreSQL conserve son empreinte.
    let token_hash = hex::encode(Sha256::digest(token.as_bytes()));

    let secure_attribute = if secure { "; Secure" } else { "" };

    let cookie = HeaderValue::from_str(&format!(
        "lostilanders_session={token}; HttpOnly; SameSite=Lax; \
         Path=/; Max-Age={SESSION_DURATION_SECONDS}{secure_attribute}"
    )).map_err(|_| ServerFnError::new("Unable to create session cookie."))?;

    sqlx::query(
        "INSERT INTO sessions (id, user_id, expires_at)
         VALUES (
             $1,
             $2,
             NOW() + ($3::bigint * INTERVAL '1 second')
         )",
    )
        .bind(&token_hash)
        .bind(user_id)
        .bind(SESSION_DURATION_SECONDS)
        .execute(pool)
        .await
        .map_err(|error| {
            eprintln!("Session insertion failed: {error}");
            ServerFnError::new("Unable to create session.")
        })?;

    context.add_response_header(SET_COOKIE, cookie);

    Ok(())
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentUser{
    pub id: i32,
    pub username: String
}

impl CurrentUser{
    pub fn from(data: (i32, String)) -> CurrentUser{
        CurrentUser{
            id: data.0,
            username: data.1
        }
    }
}

#[server]
pub async fn get_current_user() -> Result<Option<CurrentUser>, ServerFnError>{
    let cookies = get_cookies()?;

    if let Some(cookies) = cookies{
        if let Some(session) = cookies.get("lostilanders_session") {
            let pool = DB_POOL
                .get()
                .ok_or_else(|| ServerFnError::new("Database pool not initialized"))?;

            for session in session {
                let token_hash = hex::encode(Sha256::digest(session.as_bytes()));
                if let Some(user) = sqlx::query_as::<_, (i32, String)>("SELECT users.id, username FROM users LEFT JOIN sessions ON users.id=sessions.user_id AND sessions.expires_at > NOW() WHERE sessions.id=$1")
                    .bind(token_hash)
                    .fetch_optional(pool)
                    .await
                    .map_err(|e| ServerFnError::new(e.to_string()))?{
                    return Ok(Some(CurrentUser::from(user)));
                }
            }
            Ok(None)
        } else {
            Ok(None)
        }
    } else {
        Ok(None)
    }

}

#[cfg(feature = "server")]
pub fn get_cookies() -> Result<Option<HashMap<String, Vec<String>>>, ServerFnError>{
    let context = FullstackContext::current()
        .ok_or_else(|| ServerFnError::new("Request context unavailable."))?;
    let mut map: HashMap<String, Vec<String>> = HashMap::new();
    {
        let parts = context.parts_mut();

        for value in parts.headers.get_all(COOKIE).iter() {
            let header = value.to_str()
                .map_err(|_| ServerFnError::new("Invalid cookie header."))?;
            for cookie in header.split(';') {
                let cookie = cookie.trim();
                if let Some((key, value)) = cookie.split_once('='){
                    if key == "" || value == "" { continue; }
                    if map.contains_key(key){
                        map.get_mut(key).unwrap().push(value.into());
                    } else {
                        let mut v: Vec<String> = Vec::new();
                        v.push(value.into());
                        map.insert(key.into(), v);
                    }
                }
            }

        }
    }
    if !map.is_empty(){
        Ok(Some(map))
    } else {
        Ok(None)
    }
}

#[server]
pub async fn logout() -> Result<(), ServerFnError>{
    let context = FullstackContext::current()
        .ok_or_else(|| ServerFnError::new("Request context unavailable."))?;
    let cookies = get_cookies()?;
    if let Some(cookies) = cookies{
        if let Some(session) = cookies.get("lostilanders_session"){
            let pool = DB_POOL
                .get()
                .ok_or_else(|| ServerFnError::new("Database pool not initialized"))?;

            for session in session {
                let token_hash = hex::encode(Sha256::digest(session.as_bytes()));
                sqlx::query("DELETE FROM sessions WHERE id=$1")
                    .bind(token_hash)
                    .execute(pool)
                    .await
                    .map_err(|e| ServerFnError::new(e.to_string()))?;
            }
        }
    }

    let secure = secure()?;

    let secure_attribute = if secure { "; Secure" } else { "" };
    let cookie = HeaderValue::from_str(&format!(
        "lostilanders_session=; HttpOnly; SameSite=Lax; \
         Path=/; Max-Age=0{secure_attribute}"
    )).map_err(|_| ServerFnError::new("Unable to create session cookie."))?;
    context.add_response_header(SET_COOKIE, cookie);
    Ok(())
}