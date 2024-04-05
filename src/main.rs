mod settings;
mod templates;

use serde::Deserialize;
use settings::Settings;
use templates::{IndexTemplate, LoginTemplate};

use askama::Template;
use axum::{
    extract::Path, http::{header, HeaderMap, StatusCode}, response::{Html, IntoResponse, Redirect}, routing::{get, post}, Form, Router
};
use tower_sessions::{cookie::time::Duration, Expiry, MemoryStore, Session, SessionManagerLayer};
use std::{net::SocketAddr, sync::Mutex};

use tracing::{info, warn};
use tracing_subscriber::filter::EnvFilter;
use tracing_subscriber::FmtSubscriber;

#[macro_use]
extern crate lazy_static;
lazy_static! {
    static ref SETTINGS: Settings = match Settings::new() {
        Some(s) => s,
        _ => {
            warn!("Failed to parse settings, defaults will be used instead");
            Settings::from_str("").unwrap()
        }
    };
}

lazy_static! {
    static ref LOGIN_USER: Mutex<Option<LoginUser>> = Mutex::new(Option::None);
}

#[tokio::main]
async fn main() {
    // Initialize logging subsystem.
    let trace_sub = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::new("askama_axum_rust_template=debug"))
        .finish();
    tracing::subscriber::set_global_default(trace_sub).unwrap();

    let session_store = MemoryStore::default();
    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(false)
        .with_expiry(Expiry::OnInactivity(Duration::seconds(10)));

    let app = Router::new()
        .route("/", get(handle_index))
        .route("/login", get(handle_login))
        .route("/login", post(login))
        .route("/_assets/*path", get(handle_assets))
        .layer(session_layer);

    let listen_addr: SocketAddr = format!("{}:{}", SETTINGS.ip, SETTINGS.port)
        .parse()
        .unwrap();

    info!("Listening on http://{}", listen_addr);

    axum::Server::bind(&listen_addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

static THEME_CSS: &str = include_str!("../assets/theme.css");
static FAVICON: &str = include_str!("../assets/favicon.svg");

async fn handle_assets(Path(path): Path<String>) -> impl IntoResponse {
    let mut headers = HeaderMap::new();

    if path == "theme.css" {
        headers.insert(header::CONTENT_TYPE, "text/css".parse().unwrap());
        (StatusCode::OK, headers, THEME_CSS)
    } else if path == "favicon.svg" {
        (StatusCode::OK, headers, FAVICON)
    } else {
        (StatusCode::NOT_FOUND, headers, "")
    }
}

async fn handle_index() -> impl IntoResponse {
    let username_lock = LOGIN_USER.lock().unwrap();
    let is_login = (*username_lock).is_some();
    let username = if let Some(user) = username_lock.as_ref() {
        user.username.as_str()
    } else {
        ""
    };

    let template = IndexTemplate { is_login, name: username };
    let reply_html = template.render().unwrap();
    (StatusCode::OK, Html(reply_html).into_response())
}

async fn handle_login() -> impl IntoResponse {
    let template = LoginTemplate { title: "用户登录" };
    let reply_html = template.render().unwrap();
    (StatusCode::OK, Html(reply_html).into_response())
}

async fn login(session: Session, form: Form<LoginForm>) -> Redirect {
    println!("username: {}, password: {}", form.username, form.password);
    let login_user = LoginUser { username: form.username.clone() };
    let mut login_user_lock = LOGIN_USER.lock().unwrap();
    *login_user_lock = Some(login_user);
    Redirect::to("/")
}

#[derive(Deserialize)]
struct LoginForm {
    username: String,
    password: String
}

struct LoginUser {
    username: String
}
