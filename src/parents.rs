use chrono::{DateTime, TimeZone};
use chrono_tz::Europe::Paris;
use dioxus::prelude::*;
use dioxus_primitives::select::SelectGroup;
use crate::components::navbar::{Navbar, NavbarContent, NavbarItem, NavbarNav, NavbarTrigger};
use dioxus_primitives::toast::{use_toast, Toast, Toasts, ToastOptions};
use crate::{ilanders, server};
use crate::components::button::Button;
use crate::components::card::{*};
use crate::components::drag_and_drop_list::DragAndDropList;
use crate::components::input::Input;
use crate::components::label::Label;
use crate::components::select::{Select, SelectGroupLabel, SelectOption};
use crate::server::{logout, CurrentUser};

#[derive(Routable, Clone, PartialEq)]
pub enum ParentRoute {
    #[route("/")]
    Home {},
    #[route("/parents/")]
    ParentsHome {},
    #[route("/parents/ilanders-list")]
    IlandersList {},
    #[route("/parents/users-list")]
    UsersList {},
}

#[component]
fn Home() -> Element{
    rsx!{

    }
}

#[component]
fn ParentsHome() -> Element{
    rsx!{

    }
}

#[component]
fn IlandersList() -> Element {
    let ilanders = use_resource(|| async {
        server::get_ilanders().await
    });

    rsx! {
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

#[component]
fn UsersList() -> Element{
    let mut current_user = use_resource(|| async {
        server::get_current_user().await
    });
    let navigator = use_navigator();

    rsx!{
        TopBar {on_user_change: move |_| {current_user.restart();}}

        match &*current_user.read(){
            Some(Ok(Some(user))) => {
                if user.role < 2 {navigator.replace(ParentRoute::IlandersList{});}
                rsx!{
                    UsersTable { user_id_ignore: user.id }
                }
            },
            None => {rsx!{}},
            _ => {navigator.replace(ParentRoute::IlandersList{});rsx!{}}
        }
    }
}

#[component]
fn UsersTable(user_id_ignore: i32) -> Element{
    let users = use_resource(move || async move {
        server::get_all_users(user_id_ignore).await
    });
    rsx!{
        match &*users.read(){
            Some(Ok(Some(users))) => {
                let users: Vec<Element> = users
                    .iter()
                    .map(|u|rsx!{user_item{key: "{u.id}", u: u.clone()}})
                    .collect();
                rsx!{
                    DragAndDropList{
                        items: users
                    }
                }
            },
            _ => {rsx!{}}
        }
    }
}

#[component]
fn user_item(u: server::Users) -> Element {

    let mut role_save = use_signal(|| Some(u.role));

    rsx!{
        div{
            class: "users-list-item",
            p{"{u.id}"}
            p{"{u.username}"}
            p{"{u.created_at}"}
            Select::<i32>{
                value: Some(role_save.into()),
                on_value_change: move |mut value: Option<i32>|{
                    if let Some(v) = value{
                        spawn(async move {
                            match server::change_role(u.id, v).await{
                                Ok(_) => {
                                    role_save.set(Some(v));
                                }
                                Err(e) => {
                                    use_toast().error("Error".to_string(), ToastOptions::new().description(e.to_string()));
                                }
                            }
                        });
                    }
                },
                SelectGroup{
                    SelectGroupLabel{"Role"}
                    SelectOption::<i32>{
                        index: 0usize,
                        value: 0i32,
                        text_value: "User",
                        "User"
                    }
                    SelectOption::<i32>{
                        index: 1usize,
                        value: 1i32,
                        text_value: "Blog Writer",
                        "Blog Writer"
                    }
                    SelectOption::<i32>{
                        index: 2usize,
                        value: 2i32,
                        text_value: "Moderator",
                        "Moderator"
                    }
                    SelectOption::<i32>{
                        index: 3usize,
                        value: 3i32,
                        text_value: "Admin",
                        "Admin"
                    }
                    SelectOption::<i32>{
                        index: 4usize,
                        value: 4i32,
                        text_value: "Super Admin",
                        "Super Admin"
                    }
                }
            }
        }
    }
}

#[component]
fn TopBar(#[props(default)] on_user_change: EventHandler<()>) -> Element{
    let mut show_auth = use_signal(|| false);
    let mut register_mode = use_signal(|| false);
    let mut current_user = use_resource(|| async {
        server::get_current_user().await
    });


    rsx!{
        div {
            class: "page-topbar",

            div{
                Navbar{
                    NavbarItem{
                        index: 0usize,
                        value: "Ilanders List".to_string(),
                        to: ParentRoute::IlandersList {},
                        "Ilanders List"
                    }
                    match &*current_user.read(){
                        Some(Ok(Some(user))) => rsx!{
                            if user.role > 1{
                                NavbarNav{
                                    index: 1usize,
                                    NavbarTrigger { "Admin Menu" },
                                    NavbarContent{
                                        NavbarItem{
                                            index: 0usize,
                                            value: "Users List".to_string(),
                                            to: ParentRoute::UsersList{},
                                            "Users List"
                                        }
                                    }
                                }
                            }
                        },
                        _ => rsx!{}
                    }
                }
            }

            div{
                class: "login-div",

                match &*current_user.read() {
                    Some(Ok(Some(user))) => rsx!{
                        "{user.username}"
                        button {
                            class: "login-button",
                            onclick: move |_| {
                                spawn(async move {logout().await; current_user.restart(); on_user_change.call(());});
                            },
                            "Disconnect"
                        }
                    },
                    Some(Ok(None)) => rsx!{
                        button {
                            class: "login-button",
                            onclick: move |_| {
                                register_mode.set(false);
                                show_auth.set(true);
                            },
                            "Connect"
                        }
                    },
                    Some(Err(error)) => rsx!{

                    },
                    None => rsx! {
                        p { "Loading..." }
                    }
                }
            }




        }

        if show_auth() {
            AuthModal {
                on_login: move |_| current_user.restart(),

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
    on_login: EventHandler<()>,
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
                                        on_login.call(());
                                        show_auth.set(false);
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
                                        on_login.call(());
                                        show_auth.set(false);
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