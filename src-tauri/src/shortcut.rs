use std::sync::Arc;
use std::thread;
use std::time;

use rdev::{simulate, EventType, Key, SimulateError};
use tauri::App;
use tauri::AppHandle;
use tauri::Manager;
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_global_shortcut::GlobalShortcutExt;
use tauri_plugin_global_shortcut::{Shortcut, ShortcutState};
use tauri_plugin_store::{JsonValue, StoreExt};

use crate::managers::audio::AudioRecordingManager;
use crate::managers::transcription::TranscriptionManager;

fn try_send_event(event: &EventType) {
    if let Err(SimulateError) = simulate(event) {
        println!("We could not send {:?}", event);
    }
}

fn send(event: EventType) {
    try_send_event(&event);
    thread::sleep(time::Duration::from_millis(60));
}

fn send_paste() {
    // Determine the modifier key based on the OS
    #[cfg(target_os = "macos")]
    let modifier_key = Key::MetaLeft; // Command key on macOS
    #[cfg(not(target_os = "macos"))]
    let modifier_key = Key::ControlLeft; // Control key on other systems

    // Press both keys
    send(EventType::KeyPress(modifier_key));
    send(EventType::KeyPress(Key::KeyV));

    // Release both keys
    send(EventType::KeyRelease(Key::KeyV));
    send(EventType::KeyRelease(modifier_key));
}

fn paste(text: String, app_handle: AppHandle) {
    let clipboard = app_handle.clipboard();

    // get the current clipboard content
    let clipboard_content = clipboard.read_text().unwrap_or_default();

    clipboard.write_text(&text).unwrap();
    send_paste();

    // restore the clipboard
    clipboard.write_text(&clipboard_content).unwrap();
}

// const HANDY_TAURI_STORE: &str = "handy_tauri_store";

// let mut shortcut_map: HashMap<String, fn()> = HashMap::new();

fn transcribe_pressed(app: &AppHandle) {
    let rm = app.state::<Arc<AudioRecordingManager>>();
    rm.try_start_recording("transcribe");
}

fn transcribe_released(app: &AppHandle) {
    let ah = app.clone();
    let rm = Arc::clone(&app.state::<Arc<AudioRecordingManager>>());
    let tm = Arc::clone(&app.state::<Arc<TranscriptionManager>>());

    tauri::async_runtime::spawn(async move {
        if let Some(samples) = rm.stop_recording("transcribe") {
            match tm.transcribe(samples) {
                // Not .await, as transcribe is synchronous
                Ok(transcription) => {
                    println!("Global Shortcut Transcription: {}", transcription);
                    paste(transcription, ah);
                }
                Err(err) => println!("Global Shortcut Transcription error: {}", err),
            }
        }
    });
}

pub fn enable_shortcut(app: &App) {
    _register_shortcut_upon_start(
        app,
        "alt+space"
            .parse::<Shortcut>()
            .expect("Failed to parse shortcut"),
    );
}

fn _register_shortcut_upon_start(app: &App, shortcut: Shortcut) {
    // Initialize global shortcut and set its handler
    app.handle()
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(move |handler_app, scut, event| {
                    if scut == &shortcut {
                        println!("Global Shortcut pressed! {}", scut.into_string());
                        if event.state == ShortcutState::Pressed {
                            transcribe_pressed(handler_app);
                        } else if event.state == ShortcutState::Released {
                            transcribe_released(handler_app);
                        }
                    }
                })
                .build(),
        )
        .unwrap();
    app.global_shortcut().register(shortcut).unwrap(); // Register global shortcut
}
