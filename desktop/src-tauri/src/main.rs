// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod proxy;

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::Emitter;
use tauri::Manager;
use tauri_plugin_global_shortcut::{Code, Modifiers, Shortcut};

use proxy::ProxyState;

// ── Constants ─────────────────────────────────────────────────────────────

const PROXY_PORT: u16 = 19777;
const DEFAULT_TARGET_URL: &str = "http://localhost:19898";

const OVERLAY_INITIAL_WIDTH: f64 = 520.0;
const OVERLAY_INITIAL_HEIGHT: f64 = 100.0;
const OVERLAY_BOTTOM_MARGIN: f64 = 40.0;

// ── Settings helpers ──────────────────────────────────────────────────────

fn settings_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("failed to resolve app data dir: {error}"))?;
    Ok(dir.join("connection.json"))
}

fn read_settings(path: &PathBuf) -> serde_json::Value {
    fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn write_settings(path: &PathBuf, value: &serde_json::Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let contents = serde_json::to_string_pretty(value).map_err(|error| error.to_string())?;
    fs::write(path, contents).map_err(|error| error.to_string())?;
    Ok(())
}

// ── Tauri commands ────────────────────────────────────────────────────────

/// Return the proxy's target server URL (what the user wants to reach).
#[tauri::command]
fn get_target_url(app: tauri::AppHandle) -> String {
    let Ok(path) = settings_path(&app) else {
        return DEFAULT_TARGET_URL.to_string();
    };
    let settings = read_settings(&path);
    settings
        .get("server_url")
        .and_then(|v| v.as_str())
        .unwrap_or(DEFAULT_TARGET_URL)
        .to_string()
}

/// Set the proxy's target server URL and persist it.
#[tauri::command]
async fn set_target_url(
    app: tauri::AppHandle,
    proxy_state: tauri::State<'_, Arc<ProxyState>>,
    url: String,
) -> Result<(), String> {
    let path = settings_path(&app)?;
    let mut settings = read_settings(&path);
    settings["server_url"] = serde_json::Value::String(url.clone());
    write_settings(&path, &settings)?;
    *proxy_state.target_url.write().await = url;
    Ok(())
}

/// Return the stored Umbrel password (empty string if none).
#[tauri::command]
fn get_umbrel_password(app: tauri::AppHandle) -> String {
    let Ok(path) = settings_path(&app) else {
        return String::new();
    };
    let settings = read_settings(&path);
    settings
        .get("umbrel_password")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

/// Set the Umbrel password and update the proxy's auth cache.
#[tauri::command]
async fn set_umbrel_password(
    app: tauri::AppHandle,
    proxy_state: tauri::State<'_, Arc<ProxyState>>,
    password: String,
) -> Result<(), String> {
    let path = settings_path(&app)?;
    let mut settings = read_settings(&path);
    if password.is_empty() {
        settings.as_object_mut().map(|o| o.remove("umbrel_password"));
    } else {
        settings["umbrel_password"] = serde_json::Value::String(password.clone());
    }
    write_settings(&path, &settings)?;
    *proxy_state.umbrel_password.write().await = password;
    Ok(())
}

/// Return the proxy port so the frontend knows where to connect.
#[tauri::command]
fn get_proxy_port() -> u16 {
    PROXY_PORT
}

// ── Voice overlay ─────────────────────────────────────────────────────────

#[tauri::command]
fn toggle_voice_overlay(app: tauri::AppHandle) -> Result<(), String> {
    toggle_overlay(&app);
    Ok(())
}

#[tauri::command]
fn resize_overlay_window(
    app: tauri::AppHandle,
    label: String,
    width: f64,
    height: f64,
) -> Result<(), String> {
    let Some(window) = app.get_webview_window(&label) else {
        return Ok(());
    };

    let monitor = app.primary_monitor().ok().flatten();
    let screen_width = monitor
        .as_ref()
        .map(|m| m.size().width as f64 / m.scale_factor())
        .unwrap_or(1920.0);
    let screen_height = monitor
        .as_ref()
        .map(|m| m.size().height as f64 / m.scale_factor())
        .unwrap_or(1080.0);

    let x = (screen_width - width) / 2.0;
    let y = screen_height - height - OVERLAY_BOTTOM_MARGIN;

    use tauri::LogicalPosition;
    use tauri::LogicalSize;
    let _ = window.set_size(LogicalSize::new(width, height));
    let _ = window.set_position(LogicalPosition::new(x, y));

    Ok(())
}

fn activate_voice_overlay(app: &tauri::AppHandle) {
    if app.get_webview_window("voice-overlay").is_none() {
        create_overlay_window(app);
    } else if let Some(overlay) = app.get_webview_window("voice-overlay") {
        if !overlay.is_visible().unwrap_or(false) {
            apply_overlay_window_chrome(&overlay);
            let _ = overlay.show();
            let _ = overlay.set_focus();
        }
    }
}

fn toggle_overlay(app: &tauri::AppHandle) {
    if let Some(overlay) = app.get_webview_window("voice-overlay") {
        if overlay.is_visible().unwrap_or(false) {
            let _ = overlay.hide();
        } else {
            apply_overlay_window_chrome(&overlay);
            let _ = overlay.show();
            let _ = overlay.set_focus();
        }
    } else {
        create_overlay_window(app);
    }
}

fn create_overlay_window(app: &tauri::AppHandle) {
    use tauri::window::Color;
    use tauri::WebviewWindowBuilder;

    let monitor = app.primary_monitor().ok().flatten();
    let screen_width = monitor
        .as_ref()
        .map(|m| m.size().width as f64 / m.scale_factor())
        .unwrap_or(1920.0);
    let screen_height = monitor
        .as_ref()
        .map(|m| m.size().height as f64 / m.scale_factor())
        .unwrap_or(1080.0);

    let x = (screen_width - OVERLAY_INITIAL_WIDTH) / 2.0;
    let y = screen_height - OVERLAY_INITIAL_HEIGHT - OVERLAY_BOTTOM_MARGIN;

    match WebviewWindowBuilder::new(
        app,
        "voice-overlay",
        tauri::WebviewUrl::App("/overlay".into()),
    )
    .title("Voice")
    .inner_size(OVERLAY_INITIAL_WIDTH, OVERLAY_INITIAL_HEIGHT)
    .position(x, y)
    .decorations(false)
    .shadow(false)
    .transparent(true)
    .background_color(Color(0, 0, 0, 0))
    .always_on_top(true)
    .visible(true)
    .resizable(false)
    .skip_taskbar(true)
    .focused(true)
    .maximizable(false)
    .minimizable(false)
    .closable(false)
    .build()
    {
        Ok(window) => {
            apply_overlay_window_chrome(&window);
            tracing::info!("voice overlay window created");
            #[cfg(target_os = "macos")]
            {
                if let Ok(ns_window) = window.ns_window() {
                    unsafe {
                        sb_desktop_macos::lock_app_theme(1);
                    }
                    let _ = ns_window;
                }
            }
        }
        Err(error) => {
            tracing::error!(%error, "failed to create voice overlay window");
        }
    }
}

fn apply_overlay_window_chrome(window: &tauri::WebviewWindow) {
    let _ = window.set_decorations(false);
    let _ = window.set_shadow(false);
    let _ = window.set_always_on_top(true);
}

// ── Main ──────────────────────────────────────────────────────────────────

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let toggle_shortcut = Shortcut::new(Some(Modifiers::ALT), Code::Space);
    let voice_shortcut = Shortcut::new(Some(Modifiers::ALT | Modifiers::SHIFT), Code::Space);

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_shortcut(toggle_shortcut.clone())
                .unwrap()
                .with_shortcut(voice_shortcut.clone())
                .unwrap()
                .with_handler(
                    move |app, _shortcut, event| match (_shortcut, event.state) {
                        (shortcut, tauri_plugin_global_shortcut::ShortcutState::Pressed)
                            if shortcut == &toggle_shortcut =>
                        {
                            toggle_overlay(app);
                        }
                        (shortcut, tauri_plugin_global_shortcut::ShortcutState::Pressed)
                            if shortcut == &voice_shortcut =>
                        {
                            activate_voice_overlay(app);
                            let _ = app.emit("voice-overlay:start-recording", ());
                        }
                        (shortcut, tauri_plugin_global_shortcut::ShortcutState::Released)
                            if shortcut == &voice_shortcut =>
                        {
                            let _ = app.emit("voice-overlay:stop-recording", ());
                        }
                        _ => {}
                    },
                )
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            get_target_url,
            set_target_url,
            get_umbrel_password,
            set_umbrel_password,
            get_proxy_port,
            toggle_voice_overlay,
            resize_overlay_window,
        ])
        .setup(move |app| {
            // ── Load settings and start proxy ──────────────────────
            let app_handle = app.handle();
            let target_url = {
                let path = settings_path(&app_handle).ok();
                path.and_then(|p| {
                    let settings = read_settings(&p);
                    settings
                        .get("server_url")
                        .and_then(|v| v.as_str())
                        .map(String::from)
                })
                .unwrap_or_else(|| DEFAULT_TARGET_URL.to_string())
            };

            let umbrel_password = {
                let path = settings_path(&app_handle).ok();
                path.and_then(|p| {
                    let settings = read_settings(&p);
                    settings
                        .get("umbrel_password")
                        .and_then(|v| v.as_str())
                        .map(String::from)
                })
                .unwrap_or_default()
            };

            // Build shared proxy state
            let proxy_state = Arc::new(ProxyState::new(
                target_url.clone(),
                &umbrel_password,
            ));

            // Spawn the proxy server on its own tokio runtime (Tauri's
            // setup closure runs outside a tokio context).
            let state_for_proxy = proxy_state.clone();
            std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new()
                    .expect("Failed to create tokio runtime for proxy");
                rt.block_on(async {
                    proxy::start_proxy(state_for_proxy, PROXY_PORT).await;
                });
            });

            // Register in Tauri managed state
            app.manage(proxy_state);

            tracing::info!(
                "Proxy started on port {PROXY_PORT}, forwarding to {target_url}"
            );

            // ── macOS titlebar style ───────────────────────────────
            #[cfg(target_os = "macos")]
            {
                if let Some(window) = app.get_webview_window("main") {
                    match window.ns_window() {
                        Ok(ns_window) => unsafe {
                            sb_desktop_macos::set_titlebar_style(&ns_window, false);
                            sb_desktop_macos::lock_app_theme(1);
                        },
                        Err(e) => {
                            tracing::warn!("Could not get NSWindow handle: {}", e);
                        }
                    }
                }
            }

            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            #[cfg(target_os = "macos")]
            if let tauri::WindowEvent::Resized(_) = event {
                if let Ok(is_fullscreen) = window.is_fullscreen() {
                    if let Ok(ns_window) = window.ns_window() {
                        unsafe {
                            sb_desktop_macos::set_titlebar_style(&ns_window, is_fullscreen);
                        }
                    }
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error running Spacebot");
}
