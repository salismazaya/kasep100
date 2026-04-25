// use tauri_plugin_window_state::{AppHandleExt, StateFlags};
use axum::{routing::{get, post}, Json, Router, extract::State};
use local_ip_address::local_ip;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use tauri::{AppHandle, Emitter, Manager};
use tower_http::cors::CorsLayer;

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Answer {
    answer: String,
    score: i32,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct GameStateData {
    question: Option<String>,
    answers: Vec<Answer>,
}

#[derive(Clone)]
struct ServerState {
    app: AppHandle,
    game: Arc<RwLock<GameStateData>>,
}

#[derive(Deserialize)]
struct ActionPayload {
    key: String,
}

#[tauri::command]
fn get_local_ip() -> String {
    if let Ok(my_local_ip) = local_ip() {
        my_local_ip.to_string()
    } else {
        "127.0.0.1".to_string()
    }
}

async fn handle_action(State(state): State<ServerState>, Json(payload): Json<ActionPayload>) {
    let _ = state.app.emit("remote-action", payload.key);
}

async fn serve_state(State(state): State<ServerState>) -> Json<GameStateData> {
    let data = state.game.read().unwrap().clone();
    Json(data)
}

async fn serve_remote() -> axum::response::Html<&'static str> {
    axum::response::Html(include_str!("remote.html"))
}

#[tauri::command]
fn update_remote_state(state: tauri::State<Arc<RwLock<GameStateData>>>, new_state: GameStateData) {
    if let Ok(mut g) = state.write() {
        *g = new_state;
    }
}

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_window_state::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet, get_local_ip, update_remote_state])
        .setup(|app| {
            let app_handle = app.handle().clone();
            
            let game_state = Arc::new(RwLock::new(GameStateData::default()));
            app.manage(game_state.clone());
            
            tauri::async_runtime::spawn(async move {
                let server_state = ServerState {
                    app: app_handle,
                    game: game_state,
                };
            
                let router = Router::new()
                    .route("/", get(serve_remote))
                    .route("/state", get(serve_state))
                    .route("/action", post(handle_action))
                    .layer(CorsLayer::permissive())
                    .with_state(server_state);
                    
                let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
                if let Ok(listener) = tokio::net::TcpListener::bind(addr).await {
                    let _ = axum::serve(listener, router).await;
                }
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
