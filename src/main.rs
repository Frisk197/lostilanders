mod server;
mod ilanders;
mod parents;
mod components;

use std::fs;
use dioxus::prelude::*;
use crate::parents::ParentRoute;

#[cfg(feature = "server")]
use dioxus::server::axum::{
    extract::Request,
    middleware::{self, Next},
    response::{IntoResponse, Response},
};
use crate::components::toast::ToastProvider;

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
fn Stylsheets() -> Element{
    rsx!{
        Stylesheet { href: asset!("/src/components/avatar/style.css", AssetOptions::css_module()) }
        Stylesheet { href: asset!("/src/components/badge/style.css", AssetOptions::css_module()) }
        Stylesheet { href: asset!("/src/components/button/style.css", AssetOptions::css_module()) }
        Stylesheet { href: asset!("/src/components/calendar/style.css", AssetOptions::css_module()) }
        Stylesheet { href: asset!("/src/components/card/style.css", AssetOptions::css_module()) }
        Stylesheet { href: asset!("/src/components/date_picker/style.css", AssetOptions::css_module()) }
        Stylesheet { href: asset!("/src/components/drag_and_drop_list/style.css", AssetOptions::css_module()) }
        Stylesheet { href: asset!("/src/components/form/style.css", AssetOptions::css_module()) }
        Stylesheet { href: asset!("/src/components/input/style.css", AssetOptions::css_module()) }
        Stylesheet { href: asset!("/src/components/label/style.css", AssetOptions::css_module()) }
        Stylesheet { href: asset!("/src/components/navbar/style.css", AssetOptions::css_module()) }
        Stylesheet { href: asset!("/src/components/popover/style.css", AssetOptions::css_module()) }
        Stylesheet { href: asset!("/src/components/select/style.css", AssetOptions::css_module()) }
        Stylesheet { href: asset!("/src/components/textarea/style.css", AssetOptions::css_module()) }
        Stylesheet { href: asset!("/src/components/toast/style.css", AssetOptions::css_module()) }

        Stylesheet { href: asset!("/assets/dx-components-theme.css", AssetOptions::css().with_static_head(true)) }

        Stylesheet { href: asset!("/assets/main_style.css", AssetOptions::css().with_static_head(true)) }
    }
}

#[component]
fn App() -> Element {
    rsx! {
        Stylsheets {}

        ToastProvider {
            div{
                class: "page",
                Router::<ParentRoute> {}
            }
        }
    }
}