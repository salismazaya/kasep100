// use tauri_plugin_window_state::{AppHandleExt, StateFlags};
use axum::{routing::{get, post}, Json, Router, extract::State};
use local_ip_address::local_ip;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tauri::{AppHandle, Emitter, Manager};
use tower_http::cors::CorsLayer;

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

async fn handle_action(State(app): State<AppHandle>, Json(payload): Json<ActionPayload>) {
    let _ = app.emit("remote-action", payload.key);
}

async fn serve_remote() -> axum::response::Html<&'static str> {
    axum::response::Html(include_str!("remote.html"))
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
        .invoke_handler(tauri::generate_handler![greet, get_local_ip])
        .setup(|app| {
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let router = Router::new()
                    .route("/", get(serve_remote))
                    .route("/action", post(handle_action))
                    .layer(CorsLayer::permissive())
                    .with_state(app_handle);
                    
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
