use dioxus::prelude::*;
use crate::server;

#[derive(Routable, Clone, PartialEq)]
pub enum ParentRoute {
    #[route("/")]
    Home {},
}

#[component]
fn Home() -> Element {
    let mut result = use_signal(|| "Testing...".to_string());
    use_effect(move || {
        spawn(async move {
            match server::test_database().await {
                Ok(value) => result.set(format!("DB: OK ({value})")),
                Err(error) => result.set(format!("DB: ERROR - {error}")),
            }
        });
    });

    rsx! {
        h1 { "Lost iLanders" }
        p { "Bienvenue, humain !" }
        p { "{result}" }
    }
}
