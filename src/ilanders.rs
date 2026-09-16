use dioxus::prelude::*;
use dioxus::warnings::AllowFutureExt;
use serde::Deserialize;
use std::fs;
use ssh_key::PublicKey;
use crate::server;


#[cfg(feature = "server")]
pub async fn home() -> impl dioxus_server::axum::response::IntoResponse {
    (
        [("content-type", "text/markdown; charset=utf-8")],
        r#"# ***Hello ILanders!***

### Welcome to LostIlanders.com, your very own informational website built by parents.

#### Documentation:

*Create your account on the website:*
- Endpoint: https://lostilanders.com/ilanders/register
- Content-Type: application/x-www-form-urlencoded
- Method: POST
- Arguments:
	- ilander_id: Your public and unique ilander ID.
	- name: The name you use on the iland.
	- ssh_key: ssh ED25519 **PUBLIC** key you generated to later login to this website. Name it conveniently in your sandbox to not lose it.


*Login test:*
- Endpoint: https://lostilanders.com/ilanders/login_test
- Content-Type: application/x-www-form-urlencoded
- Method: POST
- Arguments:
	- ilander_id: Your public and unique ilander ID.
	- token_balance: Your token balance at the moment.
	- timestamp: The timestamp at the moment of your request.
	- private_signature: Sign \"{ilander-id}|{token-balance}|{timestamp}\" using the namespace \"lostilanders\" with your private key so we can make sure it's you."#,
    )
}


#[cfg(feature = "server")]
pub async fn health() -> impl dioxus_server::axum::response::IntoResponse {
    (
        [("content-type", "text/markdown; charset=utf-8")],
        "If you see this, the server is healthy!",
    )
}

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub ilander_id: i32,
    pub name: String,
    pub ssh_key: String,
}

#[cfg(feature = "server")]
#[derive(sqlx::FromRow)]
struct Ilander {
    id: i32,
    name: String,
    token_count: i32,
    last_connexion: f32,
    last_news_pull: f32,
}

#[cfg(feature = "server")]
pub async fn register(data: dioxus_server::axum::Form<RegisterRequest>) -> Result<(StatusCode, String), (StatusCode, String)>{
    if let Some(pool) = server::DB_POOL.get(){
        fs::write(format!("data/ssh_public_keys/{}.pub", data.ilander_id), data.ssh_key.clone())
            .map_err(|e|{
                println!("Write file error: {e}");
                (StatusCode::INTERNAL_SERVER_ERROR, "File could not be created.".into())
            })?;

        let result: bool = sqlx::query_scalar("SELECT EXISTS(SELECT ID FROM ILANDER WHERE ID=$1)")
            .bind(data.ilander_id)
            .fetch_one(pool)
            .await
            .map_err(|e| {
                println!("DB error: {e}");
                (StatusCode::INTERNAL_SERVER_ERROR, "Database error.".into())
            })?;

        if !result{
            sqlx::query("INSERT INTO ilander(ID, NAME, LAST_CONNEXION, LAST_NEWS_PULL) VALUES ($1, $2, EXTRACT(EPOCH FROM CURRENT_TIMESTAMP), NULL)")
                .bind(data.ilander_id)
                .bind(data.name.clone())
                .execute(pool)
                .await
                .map_err(|e| {
                    println!("DB error: {e}");
                    (StatusCode::INTERNAL_SERVER_ERROR, "Database error.".into())
                })?;
            Ok((StatusCode::OK, "Account created successfully!".into()))
        } else {
            Err((StatusCode::BAD_REQUEST, "Account already exists.".into()))
        }
    } else {
        Err((StatusCode::INTERNAL_SERVER_ERROR, "Could not connect to database.".into()))
    }
}


fn verify_signature(
    public_key: &PublicKey,
    message: &str,
    signature: &str,
) -> Result<(), ssh_key::Error> {
    let signature = signature.parse::<ssh_key::SshSig>()?;
    public_key.verify(
        "lostilanders",
        message.as_bytes(),
        &signature,
    )
}


#[derive(Deserialize)]
pub struct Login {
    pub ilander_id: i32,
    pub token_balance: i64,
    pub timestamp: i64,
    pub private_signature: String,
}

#[cfg(feature = "server")]
pub async fn login(login: Login) -> Result<(), (StatusCode, String)>{
    if let Ok(public_key_content) = fs::read_to_string(format!("data/ssh_public_keys/{}.pub", login.ilander_id)){
        let public_key: PublicKey = public_key_content.parse()
            .map_err(|e|{
                println!("Invalid public key: {e}");
                (StatusCode::BAD_REQUEST, "Invalid public key.".into())
            })?;
        let message = format!(
            "{}|{}|{}",
            login.ilander_id,
            login.token_balance,
            login.timestamp
        );
        if let Ok(_) = verify_signature(&public_key, &message, &login.private_signature){
            if let Some(pool) = server::DB_POOL.get(){
                let ilander_last_login: Option<i64> = sqlx::query_scalar("SELECT LAST_CONNEXION FROM ILANDER WHERE ID=$1")
                    .bind(login.ilander_id)
                    .fetch_optional(pool)
                    .await
                    .map_err(|e| {
                        println!("DB error: {e}");
                        (StatusCode::INTERNAL_SERVER_ERROR, "Database error.".into())
                    })?;
                let timestamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                if let Some(ilander_last_login) = ilander_last_login{
                    if ilander_last_login < login.timestamp && login.timestamp <= timestamp as i64{
                        sqlx::query("UPDATE ILANDER SET LAST_CONNEXION=EXTRACT(EPOCH FROM CURRENT_TIMESTAMP), TOKEN_COUNT=$1 WHERE ID=$2")
                            .bind(login.token_balance)
                            .bind(login.ilander_id)
                            .execute(pool)
                            .await
                            .map_err(|e| {
                                println!("DB error: {e}");
                                (StatusCode::INTERNAL_SERVER_ERROR, "Database error.".into())
                            })?;
                        Ok(())
                    } else {
                        Err((StatusCode::BAD_REQUEST, "Login request expired.".into()))
                    }
                } else {
                    Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Ilander {} does not exist in database.", login.ilander_id)))
                }
            } else {
                Err((StatusCode::INTERNAL_SERVER_ERROR, "Could not connect to database.".into()))
            }
        } else {
            Err((StatusCode::BAD_REQUEST, "Incorrect signature.".into()))
        }
    } else {
        Err((StatusCode::BAD_REQUEST, format!("Could not find key {}.pub. Did you register before ?", login.ilander_id)))
    }
}

#[cfg(feature = "server")]
pub async fn login_test(data: dioxus_server::axum::Form<Login>) -> Result<(StatusCode, String), (StatusCode, String)>{
    login(Login{
        ilander_id: data.ilander_id.clone(),
        token_balance: data.token_balance.clone(),
        timestamp: data.timestamp.clone(),
        private_signature: data.private_signature.clone()
    }).await?;
    if let Some(pool) = server::DB_POOL.get(){
        let name = sqlx::query_scalar::<_, String>("SELECT NAME FROM ILANDER WHERE ID=$1")
            .bind(data.ilander_id)
            .fetch_one(pool)
            .await
            .map_err(|e| {
                println!("DB error: {e}");
                (StatusCode::INTERNAL_SERVER_ERROR, "Database error.".into())
            })?;
        Ok((StatusCode::OK, format!("Welcome in {name}!")))
    } else {
        Err((StatusCode::INTERNAL_SERVER_ERROR, "Could not connect to database.".into()))
    }
}


