mod server;
mod ilanders;
mod parents;

use std::fs;
use dioxus::prelude::*;
use crate::parents::ParentRoute;

#[cfg(feature = "server")]
use dioxus::server::axum::{
    extract::Request,
    middleware::{self, Next},
    response::{IntoResponse, Response},
};

#[cfg(feature = "server")]
#[tokio::main]
async fn main() {
    fs::create_dir_all("data/ssh_public_keys").expect("Failed to create SSH public keys directory");


    use dioxus_server::{DioxusRouterExt, ServeConfig};

    let address = dioxus::cli_config::fullstack_address_or_localhost();

    let pool = server::create_db_pool()
        .await
        .expect("Failed to connect to PostgreSQL");

    let config = ServeConfig::new()
        .context(pool);

    let router = dioxus_server::axum::Router::new()
        .route("/ilander", dioxus_server::axum::routing::get(ilanders::home))
        .route("/ilander/health", dioxus_server::axum::routing::get(ilanders::health))
        .route("/ilander/register", dioxus_server::axum::routing::post(ilanders::register))
        .route("/ilander/login_test", dioxus_server::axum::routing::post(ilanders::login_test))
        .serve_dioxus_application(config, App);

    let router = router.into_make_service();

    let listener = tokio::net::TcpListener::bind(address)
        .await
        .expect("Failed to bind server");

    dioxus_server::axum::serve(listener, router)
        .await
        .expect("Server failed");
}

#[cfg(not(feature = "server"))]
fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        Router::<ParentRoute> {}
    }
}