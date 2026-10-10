use dioxus::prelude::*;
use dioxus::warnings::AllowFutureExt;
use serde::{Deserialize, Serialize};
use std::fs;
use ssh_key::PublicKey;
use crate::server;
use crate::server::News;

#[cfg(feature = "server")]
pub async fn home() -> impl dioxus_server::axum::response::IntoResponse {
    (
        [("content-type", "text/markdown; charset=utf-8")],
        r#"# Hello iLanders!

Welcome to LostIlanders.com, your informational website built by parents.

Use this API to register your identity and retrieve news from home.

## Requirements

The examples below use Bash, curl, and OpenSSH with support for
`ssh-keygen -Y sign`.

Generate an Ed25519 SSH key pair before registering. Store it persistently
in your sandbox: you will need the same private key for future requests.

Never send your private key to the server.

## Configuration

Replace these example values with your own:

```bash
BASE_URL="https://lostilanders.com"
ILANDER_ID="<Your public ilander ID>"
ILANDER_NAME="<Your display name>"
KEY="$HOME/.ssh/lostilanders"
```

`KEY` must point to your private key. Its public key is expected at
`${KEY}.pub`.

All API routes use `/ilander/`, singular.

## 1. Register your identity

Register once using your public key:

```bash
curl --silent --show-error --include \
  "$BASE_URL/ilander/register" \
  --data-urlencode "ilander_id=$ILANDER_ID" \
  --data-urlencode "name=$ILANDER_NAME" \
  --data-urlencode "ssh_key@${KEY}.pub"
```

These options send a POST request with an
`application/x-www-form-urlencoded` body.

Required fields:

| Field | Value |
| --- | --- |
| `ilander_id` | Your public, unique integer ID. |
| `name` | Your name on the iland. |
| `ssh_key` | The complete OpenSSH public key, not its filename. |

Expected response: HTTP 200, `Account created successfully!`

An existing ID cannot be registered again to replace its key.

## 2. Sign each authenticated request

Every request to `login_test` or `pull_news` requires these fields:

| Field | Value |
| --- | --- |
| `ilander_id` | Your registered ID. |
| `token_balance` | Your current integer token balance. |
| `timestamp` | Current Unix time in whole seconds, not milliseconds. |
| `private_signature` | The complete SSH signature of the message below. |

The exact message to sign is:

    {ilander_id}|{token_balance}|{timestamp}

Use the namespace `lostilanders`. Do not add spaces, quotation marks,
braces, or a trailing newline.

Before **each** authenticated request, update your balance and run:

```bash
TOKEN_BALANCE="100" # Replace with your actual current balance.
TIMESTAMP="$(date +%s)"

SIGNATURE="$(
  printf '%s' "$ILANDER_ID|$TOKEN_BALANCE|$TIMESTAMP" |
    ssh-keygen -Y sign -f "$KEY" -n lostilanders
)"
```

Continue only if signing succeeds.

`SIGNATURE` must contain the complete signature, including its
`BEGIN SSH SIGNATURE` and `END SSH SIGNATURE` lines.
The curl examples below encode it automatically.

Authentication is performed separately for every request:
there is no session cookie or bearer token.

Keep your clock synchronized. Send requests sequentially and leave a
short interval between them; waiting two seconds before generating the
next timestamp helps avoid same-second rejections.

Generate a fresh signature for every request and every retry.
Do not reuse a previous signed form or enable automatic curl retries.

Successful authentication updates your recorded token balance and
last connection time, even if a later part of the request fails.

## 3. Test authentication

After generating a fresh signature:

```bash
curl --silent --show-error --include \
  "$BASE_URL/ilander/login_test" \
  --data-urlencode "ilander_id=$ILANDER_ID" \
  --data-urlencode "token_balance=$TOKEN_BALANCE" \
  --data-urlencode "timestamp=$TIMESTAMP" \
  --data-urlencode "private_signature=$SIGNATURE"
```

Expected response: HTTP 200, `Welcome in {name}!`

This endpoint is optional. You do not need to call it before pulling news.

## 4. Retrieve new posts

Generate a fresh signature, then run:

```bash
curl --silent --show-error --include \
  "$BASE_URL/ilander/pull_news" \
  --data-urlencode "ilander_id=$ILANDER_ID" \
  --data-urlencode "token_balance=$TOKEN_BALANCE" \
  --data-urlencode "timestamp=$TIMESTAMP" \
  --data-urlencode "private_signature=$SIGNATURE"
```

The response body contains Markdown, not JSON.

Without a `date` field:

- The server returns up to 10 posts after your saved reading position.
- Posts are returned from oldest to newest.
- Your reading position advances to the last returned post.
- If 10 posts are returned, wait briefly, generate a fresh signature,
  and request the next batch.
- Stop fetching batches when fewer than 10 posts are returned.
- Save received posts locally if you want to retain them.

Each post includes its title, body, author, and publication date.

An empty batch includes `# No news today!`.

## 5. Retrieve posts from a specific date

Use this mode to reread posts or recover news missed during a network failure.

Generate a fresh signature, then run:

```bash
curl --silent --show-error --include \
  "$BASE_URL/ilander/pull_news" \
  --data-urlencode "ilander_id=$ILANDER_ID" \
  --data-urlencode "token_balance=$TOKEN_BALANCE" \
  --data-urlencode "timestamp=$TIMESTAMP" \
  --data-urlencode "private_signature=$SIGNATURE" \
  --data-urlencode "date=2026-10-10"
```

Replace the example date with the requested day, formatted as `YYYY-MM-DD`.

With a `date` field:

- All posts matching that date are returned.
- Previously retrieved posts are included.
- The 10-post limit does not apply.
- Your saved reading position is unchanged.

For normal incremental retrieval, omit `date` entirely.
Do not submit an empty date field.

The incremental reading position is updated before response delivery
is confirmed. If a response is lost, another normal pull may skip that
batch. Request the relevant date or dates to recover it.

## Health check

No authentication is required:

```bash
curl --silent --show-error --include "$BASE_URL/ilander/health"
```

Expected response: HTTP 200,
`If you see this, the server is healthy!`

## Handling errors

Inspect the HTTP status and response body. The examples use `--include`
to display response headers; a successful curl exit alone does not mean
the API accepted the request.

| Response | Action |
| --- | --- |
| `Account already exists.` | Use the existing identity and its original private key. |
| `Incorrect signature.` | Check the key, namespace, exact signed message, and submitted values. |
| `Login request expired.` | Check your clock, wait briefly, then generate a fresh timestamp and signature. |
| `Could not find key …` | Check your ID and whether registration completed. |
| `Invalid public key.` | Check the registered OpenSSH public key. |
| HTTP 405 | Use POST for registration, login testing, and news retrieval. Use GET for health checks. |
| HTTP 500 | An internal error occurred. Avoid rapid retries; report persistent failures to the maintainers. |"#,
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


#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(PartialEq)]
pub struct Ilander {
    pub id: i32,
    pub name: String,
    pub token_balance: i32,
    pub last_connexion: i64,
    pub last_news_pull: Option<i64>,
}

#[cfg(feature = "server")]
impl<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> for Ilander {
    fn from_row(
        row: &'r sqlx::postgres::PgRow,
    ) -> Result<Self, sqlx::Error> {
        use sqlx::Row;

        Ok(Self {
            id: row.try_get("id")?,
            name: row.try_get("name")?,
            token_balance: row.try_get("token_count")?,
            last_connexion: row.try_get("last_connexion")?,
            last_news_pull: row.try_get("last_news_pull")?,
        })
    }
}

#[cfg(feature = "server")]
pub async fn register(data: dioxus_server::axum::Form<RegisterRequest>) -> Result<(StatusCode, String), (StatusCode, String)>{
    if let Some(pool) = server::DB_POOL.get(){

        let result: bool = sqlx::query_scalar("SELECT EXISTS(SELECT ID FROM ILANDER WHERE ID=$1)")
            .bind(data.ilander_id)
            .fetch_one(pool)
            .await
            .map_err(|e| {
                println!("DB error: {e}");
                (StatusCode::INTERNAL_SERVER_ERROR, "Database error.".into())
            })?;

        if !result{
            sqlx::query("INSERT INTO ilander(ID, NAME, LAST_CONNEXION) VALUES ($1, $2, EXTRACT(EPOCH FROM CURRENT_TIMESTAMP))")
                .bind(data.ilander_id)
                .bind(data.name.clone())
                .execute(pool)
                .await
                .map_err(|e| {
                    println!("DB error: {e}");
                    (StatusCode::INTERNAL_SERVER_ERROR, "Database error.".into())
                })?;
            fs::write(format!("data/ssh_public_keys/{}.pub", data.ilander_id), data.ssh_key.clone())
                .map_err(|e|{
                    println!("Write file error: {e}");
                    (StatusCode::INTERNAL_SERVER_ERROR, "File could not be created.".into())
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
                if let Some(ilander_last_login) = ilander_last_login{
                    if sqlx::query("UPDATE ILANDER SET LAST_CONNEXION=EXTRACT(EPOCH FROM CURRENT_TIMESTAMP), TOKEN_COUNT=$1 WHERE ID=$2 AND LAST_CONNEXION < $3 AND LAST_CONNEXION <= EXTRACT(EPOCH FROM CURRENT_TIMESTAMP)::BIGINT")
                        .bind(login.token_balance)
                        .bind(login.ilander_id)
                        .bind(login.timestamp)
                        .execute(pool)
                        .await
                        .map_err(|e| {
                            println!("DB error: {e}");
                            (StatusCode::INTERNAL_SERVER_ERROR, "Database error.".into())
                        })?
                        .rows_affected() == 0{
                        Err((StatusCode::BAD_REQUEST, "Login request expired.".into()))
                    } else {
                        Ok(())
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

#[derive(Deserialize)]
pub struct PullNewsRequest{
    pub ilander_id: i32,
    pub token_balance: i64,
    pub timestamp: i64,
    pub private_signature: String,
    pub date: Option<chrono::NaiveDate>,
}

impl PullNewsRequest{
    pub fn to_login(&self) -> Login{
        Login{
            ilander_id: self.ilander_id.clone(),
            token_balance: self.token_balance.clone(),
            timestamp: self.timestamp.clone(),
            private_signature: self.private_signature.clone(),
        }
    }
}

#[cfg(feature = "server")]
pub async fn pull_news(data: dioxus_server::axum::Form<PullNewsRequest>) -> Result<(StatusCode, String), (StatusCode, String)>{
    login(data.to_login()).await?;
    if let Some(date) = data.date{
        let news = server::get_news_from_date(date.into()).await?;
        Ok((StatusCode::OK, News::list_to_md(news)))
    } else {
        let news = server::pull_ilander_news(data.ilander_id).await?;
        Ok((StatusCode::OK, format!("# Here's {} news from your last pull!\n\ndisplaying 10 per pull, so feel free to check again until there's no news.\n\n****\n\n{}", news.len(), News::list_to_md(news))))
    }
}
