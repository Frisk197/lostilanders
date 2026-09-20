use chrono::{DateTime, TimeZone};
use chrono_tz::Europe::Paris;
use dioxus::prelude::*;
use dioxus_primitives::toast::{use_toast, Toast, Toasts, ToastOptions};
use crate::{ilanders, server};
use crate::components::button::Button;
use crate::components::card::{*};
use crate::components::input::Input;
use crate::components::label::Label;

#[derive(Routable, Clone, PartialEq)]
pub enum ParentRoute {
    #[route("/")]
    Home {},
    #[route("/parents")]
    IlandersList {},
}

#[component]
fn Home() -> Element{
    rsx!{

    }
}

#[component]
fn IlandersList() -> Element {
    let ilanders = use_resource(|| async {
        server::get_ilanders().await
    });

    rsx! {
        document::Stylesheet {
            href: asset!("/assets/dx-components-theme.css")
        }

        document::Stylesheet {
            href: asset!("/assets/main_style.css")
        }

        div {
            class: "page",

            TopBar {}

            div {
                class: "page-header",

                h1 {
                    "Welcome to LostIlanders!"
                }

                p {
                    "Here's a list of the iLanders currently registered."
                }
            }

            div {
                class: "ilander-grid",

                match &*ilanders.read() {
                    Some(Ok(ilanders)) => rsx! {
                        for ilander in ilanders {
                            IlanderCard {
                                ilander: ilander.clone()
                            }
                        }
                    },

                    Some(Err(error)) => rsx! {
                        p { "Erreur : {error}" }
                    },

                    None => rsx! {
                        p { "Loading..." }
                    }
                }
            }
        }


    }
}

#[component]
fn TopBar() -> Element{
    let mut show_auth = use_signal(|| false);
    let mut register_mode = use_signal(|| false);

    rsx!{
        div {
            class: "page-topbar",

            button {
                class: "login-button",
                onclick: move |_| {
                    register_mode.set(false);
                    show_auth.set(true);
                },
                "Connect"
            }
        }

        if show_auth() {
            AuthModal {
                show_auth: show_auth,
                register_mode: register_mode,
            }
        }
    }
}


#[component]
fn IlanderCard(ilander: ilanders::Ilander) -> Element {
    let last_connexion = Paris
        .timestamp_opt(ilander.last_connexion, 0)
        .single()
        .map(|date| date.format("%Y-%m-%d at %H:%M").to_string())
        .unwrap_or_else(|| "Never".to_string());

    rsx! {
        Card {
            CardHeader {
                CardTitle {
                    "{ilander.name}"
                }
            }

            CardContent {
                p { "ID : {ilander.id}" }
                p { "Tokens : {ilander.token_balance}" }
            }

            CardFooter {
                p {class:"last-connexion", "Last Connection: {last_connexion}"}
            }
        }
    }
}


#[component]
fn AuthModal(
    mut show_auth: Signal<bool>,
    mut register_mode: Signal<bool>,
) -> Element {
    let mut username = use_signal(String::new);
    let mut password = use_signal(String::new);
    let mut password_confirmation = use_signal(String::new);

    rsx! {
        div {
            class: "modal-overlay",

            // Clic en dehors du popup pour fermer
            onclick: move |_| show_auth.set(false),

            div {
                class: "auth-modal",

                // Empêche le clic dans le popup de fermer celui-ci
                onclick: move |event| event.stop_propagation(),

                button {
                    class: "modal-close",

                    onclick: move |_| show_auth.set(false),

                    "×"
                }

                if register_mode() {

                    // =========================
                    // INSCRIPTION
                    // =========================

                    h2 {"Create an account"}

                    p {
                        class: "auth-description",
                        "Create your account on LostIlanders"
                    }

                    div {
                        class: "form-group",

                        Label {
                            html_for: "register_username",
                            "Username"
                        }

                        Input {
                            id: "register_username",
                            r#type: "text",
                            placeholder: "Username",
                            value: "{username}",
                            oninput: move |event: FormEvent| {
                                username.set(event.value());
                            }
                        }
                    }

                    div {
                        class: "form-group",

                        Label {
                            html_for: "register_password",
                            "Password"
                        }

                        Input {
                            id: "register_password",
                            r#type: "password",
                            placeholder: "Password",
                            value: "{password}",
                            oninput: move |event: FormEvent| {
                                password.set(event.value());
                            }
                        }
                    }

                    div {
                        class: "form-group",

                        Label {
                            html_for: "register_password_confirm",
                            "Confirm password"
                        }

                        Input {
                            id: "register_password_confirm",
                            r#type: "password",
                            placeholder: "Confirm password",
                            value: "{password_confirmation}",
                            oninput: move |event: FormEvent| {
                                password_confirmation.set(event.value());
                            }
                        }
                    }

                    Button {
                        class: "auth-submit",

                        onclick: move |_| {
                            let username = username();
                            let password = password();
                            let password_confirm = password_confirmation();

                            spawn(async move {
                                if password != password_confirm {
                                    use_toast().error("Passwords must match!".to_string(),
                                            ToastOptions::new());
                                    return;
                                }

                                match server::register(username, password).await {
                                    Ok(_) => {
                                        use_toast().success("Account created!".to_string(),
                                            ToastOptions::new());
                                    }

                                    Err(error) => {
                                        use_toast().error("Account creation error.".to_string(),
                                            ToastOptions::new()
                                            .description(error.to_string()));
                                    }
                                }
                            });
                        },

                        "Sign up"
                    }

                    p {
                        class: "auth-switch",

                        "Do you have an account ? "

                        Button {
                            onclick: move |_| {
                                register_mode.set(false);
                            },

                            "Connect"
                        }
                    }

                } else {

                    // =========================
                    // CONNEXION
                    // =========================

                    h2 {"Connection"}

                    p {
                        class: "auth-description",
                        "Sign into your LostIlanders account"
                    }

                    div {
                        class: "form-group",

                        Label {
                            html_for: "connexion_username",
                            "Username"
                        }

                        Input {
                            id: "connexion_username",
                            r#type: "text",
                            placeholder: "Username",
                            value: "{username}",
                            oninput: move |event: FormEvent| {
                                username.set(event.value());
                            }
                        }
                    }

                    div {
                        class: "form-group",

                        Label {
                            html_for: "connexion_password",
                            "Password"
                        }

                        Input {
                            id: "connexion_password",
                            r#type: "password",
                            placeholder: "Password",
                            value: "{password}",
                            oninput: move |event: FormEvent| {
                                password.set(event.value());
                            }
                        }
                    }

                    Button {
                        class: "auth-submit",

                        onclick: move |_| {
                            let username = username();
                            let password = password();

                            spawn(async move {
                                match server::login(username, password).await {
                                    Ok(_) => {
                                        use_toast().success("You are connected!".to_string(),
                                            ToastOptions::new());
                                    }

                                    Err(error) => {
                                        use_toast().error("Login error".to_string(),
                                            ToastOptions::new()
                                            .description(error.to_string()));
                                    }
                                }
                            });
                        },

                        "Connect"
                    }

                    p {
                        class: "auth-switch",

                        "No account yet ? "

                        Button {
                            onclick: move |_| {
                                register_mode.set(true);
                            },

                            "Sign up"
                        }
                    }
                }
            }
        }
    }
}