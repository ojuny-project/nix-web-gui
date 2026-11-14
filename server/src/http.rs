use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::{Request, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use futures::Stream;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::{broadcast, Mutex as AsyncMutex};
use tower::ServiceExt;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;

use nix_web_gui_engine::session::SettingsSession;
use nix_web_gui_engine::theme::{ThemeState, ThemeStore};

use crate::auth;
use crate::auth::session::{Session, SessionStore};
use crate::config::AppConfig;
use crate::rpc;
use crate::worker_process::Worker;

const COOKIE: &str = "nix_web_gui_session";

#[derive(Clone)]
pub enum EngineMode {
    InProcess(Arc<AsyncMutex<HashMap<String, Option<SettingsSession>>>>),
    Workers(Arc<AsyncMutex<HashMap<String, Arc<AsyncMutex<Worker>>>>>),
}

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AppConfig>,
    pub sessions: Arc<SessionStore>,
    pub theme: Arc<ThemeStore>,
    pub engine: EngineMode,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/health", get(health))
        .route("/api/theme", get(get_theme))
        .route("/api/state", get(get_state))
        .route("/api/me", get(me))
        .route("/api/login", post(login))
        .route("/api/logout", post(logout))
        .route("/api/rpc", post(rpc))
        .route("/api/events", get(events))
        .fallback(static_handler)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

#[derive(Debug)]
struct ApiError(StatusCode, String);

impl ApiError {
    fn bad_request(message: impl Into<String>) -> Self {
        Self(StatusCode::BAD_REQUEST, message.into())
    }

    fn unauthorized(message: impl Into<String>) -> Self {
        Self(StatusCode::UNAUTHORIZED, message.into())
    }

    fn forbidden(message: impl Into<String>) -> Self {
        Self(StatusCode::FORBIDDEN, message.into())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.0, Json(json!({ "error": self.1 }))).into_response()
    }
}

async fn health() -> Json<Value> {
    Json(json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

async fn get_theme(State(state): State<AppState>) -> Json<ThemeState> {
    Json(state.theme.current())
}

#[derive(Debug, Deserialize)]
struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Debug, Serialize)]
struct LoginResponse {
    username: String,
    csrf: String,
}

async fn login(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(body): Json<LoginRequest>,
) -> Result<(CookieJar, Json<LoginResponse>), ApiError> {
    let username = body.username.trim();
    if username.is_empty() || body.password.is_empty() {
        return Err(ApiError::bad_request("username and password are required"));
    }

    auth::authenticate(&state.config, username, &body.password)
        .map_err(|error| ApiError::unauthorized(error.to_string()))?;

    state.sessions.purge_expired();
    let (id, csrf) = state.sessions.create(username, state.config.session_ttl);
    let cookie = Cookie::build((COOKIE, id))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Strict)
        .build();

    Ok((
        jar.add(cookie),
        Json(LoginResponse {
            username: username.to_string(),
            csrf,
        }),
    ))
}

async fn logout(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<(CookieJar, Json<Value>), ApiError> {
    if let Some(cookie) = jar.get(COOKIE) {
        let id = cookie.value().to_string();
        state.sessions.remove(&id);
        match &state.engine {
            EngineMode::InProcess(settings) => {
                settings.lock().await.remove(&id);
            }
            EngineMode::Workers(workers) => {
                workers.lock().await.remove(&id);
            }
        }
    }
    let jar = jar.remove(Cookie::from(COOKIE));
    Ok((jar, Json(json!({ "ok": true }))))
}

async fn me(State(state): State<AppState>, jar: CookieJar) -> Result<Json<Value>, ApiError> {
    let session = require_session(&state, &jar)?;
    Ok(Json(json!({
        "username": session.username,
        "csrf": session.csrf,
    })))
}

async fn get_state(State(state): State<AppState>, jar: CookieJar) -> Result<Json<Value>, ApiError> {
    require_session(&state, &jar)?;
    let config = nix_web_gui_engine::config::WebConfig::load();
    Ok(Json(json!({
        "flake": state.config.flake.clone().or(config.flake_path.clone()),
        "configFlake": config.flake_path,
        "recentFlakes": config.recent_flake_paths,
        "hostname": hostname(),
        "page": state.config.startup_page,
        "auth": if state.config.dev_auth { "dev" } else { "pam" },
        "theme": state.theme.current(),
    })))
}

fn hostname() -> Option<String> {
    std::fs::read_to_string("/etc/hostname")
        .ok()
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
}

fn session_id(jar: &CookieJar) -> Option<String> {
    jar.get(COOKIE).map(|cookie| cookie.value().to_string())
}

fn require_session(state: &AppState, jar: &CookieJar) -> Result<Session, ApiError> {
    let id = session_id(jar).ok_or_else(|| ApiError::unauthorized("not signed in"))?;
    state
        .sessions
        .get(&id)
        .ok_or_else(|| ApiError::unauthorized("session expired"))
}

#[derive(Debug, Deserialize)]
struct RpcRequest {
    #[serde(default)]
    id: Value,
    method: String,
    #[serde(default)]
    params: Value,
}

async fn rpc(
    State(state): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(request): Json<RpcRequest>,
) -> Result<Json<Value>, ApiError> {
    let id = session_id(&jar).ok_or_else(|| ApiError::unauthorized("not signed in"))?;
    let session = state
        .sessions
        .get(&id)
        .ok_or_else(|| ApiError::unauthorized("session expired"))?;

    let csrf = headers
        .get("x-csrf-token")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    if !state.sessions.validate_csrf(&id, csrf) {
        return Err(ApiError::forbidden("missing or invalid CSRF token"));
    }

    let method = request.method;
    let request_id = request.id;
    let request_value = json!({
        "jsonrpc": "2.0",
        "id": request_id.clone(),
        "method": method.clone(),
        "params": request.params.clone(),
    });

    let response = if method == "system.ping" {
        json!({
            "jsonrpc": "2.0",
            "id": request_id,
            "result": { "pong": true, "user": session.username },
        })
    } else if method == "system.capabilities" {
        json!({
            "jsonrpc": "2.0",
            "id": request_id,
            "result": {
                "apiVersion": nix_web_gui_engine::API_VERSION,
                "engine": "nix-settings-gui",
                "ojunyWidgets": cfg!(feature = "ojuny-widgets"),
            },
        })
    } else if let Some(result) = secureboot_action(&method, &request.params) {
        match result {
            Ok(value) => json!({
                "jsonrpc": "2.0",
                "id": request_id,
                "result": value,
            }),
            Err(message) => json!({
                "jsonrpc": "2.0",
                "id": request_id,
                "error": { "code": -32012, "message": message },
            }),
        }
    } else {
        match &state.engine {
            EngineMode::InProcess(settings) => {
                let mut guard = settings.lock().await;
                let settings = guard.entry(id.clone()).or_insert(None);
                rpc::handle(settings, &request_value)
            }
            EngineMode::Workers(workers) => {
                let worker = {
                    let mut guard = workers.lock().await;
                    if let Some(worker) = guard.get(&id) {
                        worker.clone()
                    } else {
                        let worker = Arc::new(AsyncMutex::new(
                            Worker::spawn(&session.username).map_err(|error| {
                                ApiError(StatusCode::BAD_GATEWAY, error)
                            })?,
                        ));
                        guard.insert(id.clone(), worker.clone());
                        worker
                    }
                };
                let mut worker = worker.lock().await;
                let result = worker.request(&request_value).await;
                drop(worker);
                if result.is_err() {
                    workers.lock().await.remove(&id);
                }
                result.map_err(|error| ApiError(StatusCode::BAD_GATEWAY, error))?
            }
        }
    };

    Ok(Json(response))
}

fn secureboot_action(method: &str, params: &Value) -> Option<Result<Value, String>> {
    match method {
        "secureboot.setup" => Some(nix_web_gui_engine::secureboot::setup(
            params
                .get("keyAction")
                .and_then(Value::as_str)
                .unwrap_or("keep"),
            params
                .get("microsoft")
                .and_then(Value::as_bool)
                .unwrap_or(true),
            params
                .get("firmwareBuiltin")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        )),
        "secureboot.rebuild" => {
            let root = params.get("root").and_then(Value::as_str).unwrap_or_default();
            if root.is_empty() {
                return Some(Err("root is required".to_string()));
            }
            Some(
                nix_web_gui_engine::secureboot::rebuild(root)
                    .map(|output| json!({ "output": output })),
            )
        }
        "secureboot.reboot" => Some(
            if params.get("confirm").and_then(Value::as_bool) == Some(true) {
                nix_web_gui_engine::secureboot::reboot().map(|_| json!({ "rebooting": true }))
            } else {
                Err("reboot requires confirm: true".to_string())
            },
        ),
        _ => None,
    }
}

async fn events(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, ApiError> {
    require_session(&state, &jar)?;

    let initial = state.theme.current();
    let mut receiver = state.theme.subscribe();
    let stream = async_stream::stream! {
        yield event("theme", &initial);
        let mut heartbeat = tokio::time::interval(Duration::from_secs(15));
        loop {
            tokio::select! {
                _ = heartbeat.tick() => {
                    yield Ok(Event::default().event("heartbeat").data("{}"));
                }
                changed = receiver.recv() => {
                    match changed {
                        Ok(theme) => yield event("theme.changed", &theme),
                        Err(broadcast::error::RecvError::Lagged(_)) => continue,
                        Err(broadcast::error::RecvError::Closed) => break,
                    }
                }
            }
        }
    };

    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
}

fn event(name: &str, theme: &ThemeState) -> Result<Event, Infallible> {
    let data = serde_json::to_string(theme).unwrap_or_else(|_| "{}".to_string());
    Ok(Event::default().event(name).data(data))
}

async fn static_handler(State(state): State<AppState>, request: Request) -> Response {
    let index = state.config.static_dir.join("index.html");
    if index.is_file() {
        return ServeDir::new(&state.config.static_dir)
            .fallback(ServeFile::new(index))
            .oneshot(request)
            .await
            .into_response();
    }

    Html(PLACEHOLDER).into_response()
}

const PLACEHOLDER: &str = r#"<!doctype html>
<html lang="en">
<head><meta charset="utf-8"><title>nix-web-gui</title></head>
<body style="font-family: system-ui; background: #272e33; color: #d3c6aa; padding: 2rem">
<h1>nix-web-gui</h1>
<p>The frontend is not built. Run <code>npm ci &amp;&amp; npm run build</code> in <code>frontend/</code>.</p>
</body>
</html>"#;

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use std::time::Duration;
    use tower::ServiceExt;

    fn test_state() -> AppState {
        let theme_path = std::env::temp_dir().join(format!(
            "nix-web-gui-http-theme-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::write(&theme_path, "catppuccin-mocha\n").unwrap();

        AppState {
            config: Arc::new(AppConfig {
                listen: "127.0.0.1:0".parse().unwrap(),
                static_dir: ".".into(),
                theme_file: theme_path.clone(),
                flake: None,
                startup_page: None,
                pam_service: "nix-web-gui".into(),
                allowed_groups: Vec::new(),
                dev_auth: true,
                dev_password: Some("secret".into()),
                session_ttl: Duration::from_secs(60),
            }),
            sessions: Arc::new(SessionStore::default()),
            theme: ThemeStore::new(theme_path),
            engine: EngineMode::InProcess(Arc::new(AsyncMutex::new(HashMap::new()))),
        }
    }

    fn login_request() -> Request {
        Request::builder()
            .method("POST")
            .uri("/api/login")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"username":"den","password":"secret"}"#))
            .unwrap()
    }

    #[tokio::test]
    async fn theme_endpoint_returns_the_active_name() {
        let response = router(test_state())
            .oneshot(
                Request::builder()
                    .uri("/api/theme")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let value: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(value["name"], "catppuccin-mocha");
    }

    #[tokio::test]
    async fn login_sets_cookie_and_rpc_requires_csrf() {
        let app = router(test_state());
        let login = app.clone().oneshot(login_request()).await.unwrap();
        assert_eq!(login.status(), StatusCode::OK);

        let cookie = login
            .headers()
            .get("set-cookie")
            .unwrap()
            .to_str()
            .unwrap()
            .split(';')
            .next()
            .unwrap()
            .to_string();
        let body = axum::body::to_bytes(login.into_body(), usize::MAX)
            .await
            .unwrap();
        let value: Value = serde_json::from_slice(&body).unwrap();
        let csrf = value["csrf"].as_str().unwrap();

        let rpc_body =
            r#"{"jsonrpc":"2.0","id":1,"method":"system.ping","params":{}}"#;
        let denied = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/rpc")
                    .header("content-type", "application/json")
                    .header("cookie", &cookie)
                    .body(Body::from(rpc_body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(denied.status(), StatusCode::FORBIDDEN);

        let allowed = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/rpc")
                    .header("content-type", "application/json")
                    .header("cookie", &cookie)
                    .header("x-csrf-token", csrf)
                    .body(Body::from(rpc_body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(allowed.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn wrong_password_is_rejected() {
        let response = router(test_state())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/login")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"username":"den","password":"wrong"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}

#[cfg(test)]
mod engine_tests {
    use super::*;
    use axum::body::Body;
    use std::time::Duration;
    use tower::ServiceExt;

    fn fixture_root() -> String {
        format!(
            "{}/../tests/fixtures/plain-flake",
            env!("CARGO_MANIFEST_DIR")
        )
    }

    fn state() -> AppState {
        AppState {
            config: Arc::new(AppConfig {
                listen: "127.0.0.1:0".parse().unwrap(),
                static_dir: ".".into(),
                theme_file: "/tmp/nix-web-gui-engine-test-theme".into(),
                flake: None,
                startup_page: None,
                pam_service: "nix-web-gui".into(),
                allowed_groups: Vec::new(),
                dev_auth: true,
                dev_password: Some("secret".into()),
                session_ttl: Duration::from_secs(60),
            }),
            sessions: Arc::new(SessionStore::default()),
            theme: ThemeStore::new("/tmp/nix-web-gui-engine-test-theme".into()),
            engine: EngineMode::InProcess(Arc::new(AsyncMutex::new(HashMap::new()))),
        }
    }

    async fn login(app: &Router) -> (String, String) {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/login")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"username":"den","password":"secret"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        let cookie = response
            .headers()
            .get("set-cookie")
            .unwrap()
            .to_str()
            .unwrap()
            .split(';')
            .next()
            .unwrap()
            .to_string();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let value: Value = serde_json::from_slice(&body).unwrap();
        (cookie, value["csrf"].as_str().unwrap().to_string())
    }

    async fn rpc(
        app: &Router,
        cookie: &str,
        csrf: &str,
        method: &str,
        params: Value,
    ) -> Value {
        let body = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params,
        })
        .to_string();
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/rpc")
                    .header("content-type", "application/json")
                    .header("cookie", cookie)
                    .header("x-csrf-token", csrf)
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        serde_json::from_slice(&body).unwrap()
    }

    #[tokio::test]
    async fn opens_a_fixture_and_stages_an_edit() {
        let app = router(state());
        let (cookie, csrf) = login(&app).await;

        let opened = rpc(
            &app,
            &cookie,
            &csrf,
            "session.open",
            json!({ "root": fixture_root() }),
        )
        .await;
        assert!(opened["result"]["menus"].as_u64().unwrap() >= 1);
        assert!(opened["result"]["records"].as_u64().unwrap() >= 3);

        let pages = rpc(&app, &cookie, &csrf, "page.list", json!({})).await;
        assert_eq!(
            pages["result"]["pages"][0]["title"],
            Value::String("System".to_string())
        );

        let staged = rpc(
            &app,
            &cookie,
            &csrf,
            "control.set",
            json!({ "path": "networking.hostName", "value": "\"web\"" }),
        )
        .await;
        assert_eq!(staged["result"]["staged"], true);

        let preview = rpc(&app, &cookie, &csrf, "edits.preview", json!({})).await;
        assert_eq!(preview["result"]["edits"][0]["after"], "\"web\"");
    }
}
