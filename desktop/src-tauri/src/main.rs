// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod proxy;

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Mutex};
use tauri::Emitter;
use tauri::Manager;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut};

use proxy::ProxyState;

// ── Constants ─────────────────────────────────────────────────────────────

const PROXY_PORT: u16 = 19777;
const DEFAULT_TARGET_URL: &str = "http://localhost:19898";

// ── Global shortcut state ─────────────────────────────────────────────────

static VOICE_SHORTCUT_ENABLED: AtomicBool = AtomicBool::new(true);
static VOICE_RECORDING_ENABLED: AtomicBool = AtomicBool::new(true);

static CURRENT_TOGGLE_KEY: LazyLock<Mutex<String>> =
    LazyLock::new(|| Mutex::new(String::new()));
static PARSED_TOGGLE: LazyLock<Mutex<Option<Shortcut>>> =
    LazyLock::new(|| Mutex::new(None));

static CURRENT_RECORDING_KEY: LazyLock<Mutex<String>> =
    LazyLock::new(|| Mutex::new(String::new()));
static PARSED_RECORDING: LazyLock<Mutex<Option<Shortcut>>> =
    LazyLock::new(|| Mutex::new(None));

const DEFAULT_TOGGLE_KEY: &str = "Alt+Space";
const DEFAULT_RECORDING_KEY: &str = "Alt+Shift+Space";

fn read_bool_inv(settings: &serde_json::Value, key: &str, default_enabled: bool) -> bool {
    !settings
        .get(key)
        .and_then(|v| v.as_bool())
        .unwrap_or(!default_enabled)
}

fn read_str_setting(settings: &serde_json::Value, key: &str, default: &str) -> String {
    settings
        .get(key)
        .and_then(|v| v.as_str())
        .unwrap_or(default)
        .to_string()
}

fn parse_key_code(s: &str) -> Option<Code> {
    Some(match s {
        "Space" => Code::Space,
        "Enter" => Code::Enter,
        "Tab" => Code::Tab,
        "Escape" => Code::Escape,
        "Backspace" => Code::Backspace,
        "Delete" => Code::Delete,
        "Home" => Code::Home,
        "End" => Code::End,
        "PageUp" => Code::PageUp,
        "PageDown" => Code::PageDown,
        "ArrowUp" => Code::ArrowUp,
        "ArrowDown" => Code::ArrowDown,
        "ArrowLeft" => Code::ArrowLeft,
        "ArrowRight" => Code::ArrowRight,
        "F1" => Code::F1,   "F2" => Code::F2,   "F3" => Code::F3,
        "F4" => Code::F4,   "F5" => Code::F5,   "F6" => Code::F6,
        "F7" => Code::F7,   "F8" => Code::F8,   "F9" => Code::F9,
        "F10" => Code::F10, "F11" => Code::F11, "F12" => Code::F12,
        "F13" => Code::F13, "F14" => Code::F14, "F15" => Code::F15,
        "F16" => Code::F16, "F17" => Code::F17, "F18" => Code::F18,
        "F19" => Code::F19, "F20" => Code::F20,
        "0" => Code::Digit0, "1" => Code::Digit1, "2" => Code::Digit2,
        "3" => Code::Digit3, "4" => Code::Digit4, "5" => Code::Digit5,
        "6" => Code::Digit6, "7" => Code::Digit7, "8" => Code::Digit8,
        "9" => Code::Digit9,
        "A" => Code::KeyA, "B" => Code::KeyB, "C" => Code::KeyC,
        "D" => Code::KeyD, "E" => Code::KeyE, "F" => Code::KeyF,
        "G" => Code::KeyG, "H" => Code::KeyH, "I" => Code::KeyI,
        "J" => Code::KeyJ, "K" => Code::KeyK, "L" => Code::KeyL,
        "M" => Code::KeyM, "N" => Code::KeyN, "O" => Code::KeyO,
        "P" => Code::KeyP, "Q" => Code::KeyQ, "R" => Code::KeyR,
        "S" => Code::KeyS, "T" => Code::KeyT, "U" => Code::KeyU,
        "V" => Code::KeyV, "W" => Code::KeyW, "X" => Code::KeyX,
        "Y" => Code::KeyY, "Z" => Code::KeyZ,
        "Comma" => Code::Comma,
        "Period" => Code::Period,
        "Minus" => Code::Minus,
        "Equal" => Code::Equal,
        "BracketLeft" => Code::BracketLeft,
        "BracketRight" => Code::BracketRight,
        "Semicolon" => Code::Semicolon,
        "Quote" => Code::Quote,
        "Backslash" => Code::Backslash,
        "Backquote" => Code::Backquote,
        "Slash" => Code::Slash,
        "IntlBackslash" => Code::IntlBackslash,
        _ => return None,
    })
}

fn parse_shortcut(s: &str) -> Option<Shortcut> {
    let parts: Vec<&str> = s.split('+').collect();
    if parts.len() < 2 {
        return None;
    }
    let code_str = parts.last()?;
    let mut modifiers = Modifiers::empty();
    for part in parts[..parts.len() - 1].iter() {
        match *part {
            "Alt" | "Option" => modifiers |= Modifiers::ALT,
            "Cmd" | "Command" | "Super" => modifiers |= Modifiers::SUPER,
            "Shift" => modifiers |= Modifiers::SHIFT,
            "Ctrl" | "Control" => modifiers |= Modifiers::CONTROL,
            _ => return None,
        }
    }
    let code = parse_key_code(code_str)?;
    Some(Shortcut::new(Some(modifiers), code))
}

fn register_shortcut(
    app: &tauri::AppHandle,
    stored: &std::sync::Mutex<Option<Shortcut>>,
    key_str: &str,
) {
    if let Some(shortcut) = parse_shortcut(key_str) {
        *stored.lock().unwrap() = Some(shortcut.clone());
        if let Err(e) = app.global_shortcut().register(shortcut) {
            tracing::warn!(%e, "Failed to register shortcut {key_str}");
        }
    }
}

fn unregister_shortcut(app: &tauri::AppHandle, key_str: &str) {
    if let Some(shortcut) = parse_shortcut(key_str) {
        let _ = app.global_shortcut().unregister(shortcut);
    }
}

fn swap_shortcut(
    app: &tauri::AppHandle,
    key_mutex: &std::sync::Mutex<String>,
    parsed_mutex: &std::sync::Mutex<Option<Shortcut>>,
    new_key: &str,
) -> Result<(), String> {
    if parse_shortcut(new_key).is_none() {
        return Err(format!("Invalid shortcut key: {new_key}"));
    }
    let old_key = {
        let mut current = key_mutex.lock().map_err(|e| e.to_string())?;
        let old = current.clone();
        *current = new_key.to_string();
        old
    };
    unregister_shortcut(app, &old_key);
    register_shortcut(app, parsed_mutex, new_key);
    Ok(())
}

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

// ── Shortcut config ──────────────────────────────────────────────────────

const SHORTCUT_DISABLED_KEY: &str = "voice_shortcut_disabled";
const SHORTCUT_TOGGLE_KEY: &str = "voice_shortcut_toggle";
const RECORDING_DISABLED_KEY: &str = "voice_recording_disabled";
const RECORDING_TOGGLE_KEY: &str = "voice_recording_shortcut";

/// — Toggle shortcut (show/hide overlay) —

#[tauri::command]
fn get_toggle_shortcut_state(app: tauri::AppHandle) -> bool {
    let Ok(path) = settings_path(&app) else {
        return true;
    };
    let settings = read_settings(&path);
    read_bool_inv(&settings, SHORTCUT_DISABLED_KEY, true)
}

#[tauri::command]
fn get_toggle_shortcut_key(app: tauri::AppHandle) -> String {
    let Ok(path) = settings_path(&app) else {
        return DEFAULT_TOGGLE_KEY.to_string();
    };
    let settings = read_settings(&path);
    read_str_setting(&settings, SHORTCUT_TOGGLE_KEY, DEFAULT_TOGGLE_KEY)
}

#[tauri::command]
async fn set_toggle_shortcut(
    app: tauri::AppHandle,
    enabled: bool,
    key: String,
) -> Result<(), String> {
    let path = settings_path(&app)?;
    let mut settings = read_settings(&path);
    settings[SHORTCUT_DISABLED_KEY] = serde_json::Value::Bool(!enabled);
    settings[SHORTCUT_TOGGLE_KEY] = serde_json::Value::String(key.clone());
    write_settings(&path, &settings)?;
    VOICE_SHORTCUT_ENABLED.store(enabled, Ordering::SeqCst);
    swap_shortcut(&app, &*CURRENT_TOGGLE_KEY, &*PARSED_TOGGLE, &key)?;
    Ok(())
}

/// — Recording shortcut (start/stop recording) —

#[tauri::command]
fn get_recording_shortcut_state(app: tauri::AppHandle) -> bool {
    let Ok(path) = settings_path(&app) else {
        return true;
    };
    let settings = read_settings(&path);
    read_bool_inv(&settings, RECORDING_DISABLED_KEY, true)
}

#[tauri::command]
fn get_recording_shortcut_key(app: tauri::AppHandle) -> String {
    let Ok(path) = settings_path(&app) else {
        return DEFAULT_RECORDING_KEY.to_string();
    };
    let settings = read_settings(&path);
    read_str_setting(&settings, RECORDING_TOGGLE_KEY, DEFAULT_RECORDING_KEY)
}

#[tauri::command]
async fn set_recording_shortcut(
    app: tauri::AppHandle,
    enabled: bool,
    key: String,
) -> Result<(), String> {
    let path = settings_path(&app)?;
    let mut settings = read_settings(&path);
    settings[RECORDING_DISABLED_KEY] = serde_json::Value::Bool(!enabled);
    settings[RECORDING_TOGGLE_KEY] = serde_json::Value::String(key.clone());
    write_settings(&path, &settings)?;
    VOICE_RECORDING_ENABLED.store(enabled, Ordering::SeqCst);
    swap_shortcut(&app, &*CURRENT_RECORDING_KEY, &*PARSED_RECORDING, &key)?;
    Ok(())
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

/// Set the macOS native window appearance. `theme_type` is:
/// - `0` = light (aqua)
/// - `1` = dark (darkAqua)
/// - `-1` = follow system (nil)
#[tauri::command]
fn set_native_theme(theme_type: isize) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    unsafe {
        sb_desktop_macos::lock_app_theme(theme_type);
    }
    Ok(())
}

/// Return the system-level appearance: 0 for light (aqua), 1 for dark (darkAqua).
/// Ignores any per-app override so the frontend can detect the true system
/// preference even when the app's NSAppearance is locked.
#[tauri::command]
fn get_system_appearance() -> isize {
    #[cfg(target_os = "macos")]
    unsafe {
        return sb_desktop_macos::get_system_appearance();
    }
    #[cfg(not(target_os = "macos"))]
    1
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
                        sb_desktop_macos::lock_app_theme(-1);
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

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(
                    move |app, shortcut, event| {
                        let pressed = event.state
                            == tauri_plugin_global_shortcut::ShortcutState::Pressed;
                        let released = event.state
                            == tauri_plugin_global_shortcut::ShortcutState::Released;

                        // Check toggle shortcut
                        if pressed || released {
                            if let Ok(guard) = PARSED_TOGGLE.lock() {
                                if let Some(ref toggle) = *guard {
                                    if shortcut == toggle {
                                        if VOICE_SHORTCUT_ENABLED.load(Ordering::Relaxed) {
                                            if pressed {
                                                toggle_overlay(app);
                                            }
                                        }
                                        return;
                                    }
                                }
                            }
                        }

                        // Check recording shortcut
                        if let Ok(guard) = PARSED_RECORDING.lock() {
                            if let Some(ref recording) = *guard {
                                if shortcut == recording {
                                    if VOICE_RECORDING_ENABLED.load(Ordering::Relaxed) {
                                        if pressed {
                                            activate_voice_overlay(app);
                                            let _ = app.emit("voice-overlay:start-recording", ());
                                        } else if released {
                                            let _ = app.emit("voice-overlay:stop-recording", ());
                                        }
                                    }
                                    return;
                                }
                            }
                        }
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
            set_native_theme,
            get_system_appearance,
            get_toggle_shortcut_state,
            get_toggle_shortcut_key,
            set_toggle_shortcut,
            get_recording_shortcut_state,
            get_recording_shortcut_key,
            set_recording_shortcut,
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

            // Read shortcut config from settings & register both shortcuts
            {
                let path = settings_path(&app_handle).ok();
                let settings = path.as_ref().map(read_settings).unwrap_or_default();

                let toggle_enabled = read_bool_inv(&settings, SHORTCUT_DISABLED_KEY, true);
                let toggle_key = read_str_setting(&settings, SHORTCUT_TOGGLE_KEY, DEFAULT_TOGGLE_KEY);
                VOICE_SHORTCUT_ENABLED.store(toggle_enabled, Ordering::SeqCst);
                *CURRENT_TOGGLE_KEY.lock().unwrap() = toggle_key.clone();
                register_shortcut(app.handle(), &*PARSED_TOGGLE, &toggle_key);

                let rec_enabled = read_bool_inv(&settings, RECORDING_DISABLED_KEY, true);
                let rec_key = read_str_setting(&settings, RECORDING_TOGGLE_KEY, DEFAULT_RECORDING_KEY);
                VOICE_RECORDING_ENABLED.store(rec_enabled, Ordering::SeqCst);
                *CURRENT_RECORDING_KEY.lock().unwrap() = rec_key.clone();
                register_shortcut(app.handle(), &*PARSED_RECORDING, &rec_key);
            }

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
                            sb_desktop_macos::lock_app_theme(-1);
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
