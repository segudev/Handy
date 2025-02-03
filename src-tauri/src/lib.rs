mod managers;

use managers::keybinding::KeyBindingManager;
use managers::transcription::TranscriptionManager;
use managers::{audio::AudioRecordingManager, transcription};
use rdev::Key;
use std::sync::{Arc, Mutex};
use tauri_plugin_autostart::MacosLauncher;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let recording_manager =
        Arc::new(AudioRecordingManager::new().expect("Failed to initialize recording manager"));
    let transcription_manager =
        Arc::new(TranscriptionManager::new().expect("Failed to initialize transcription manager"));
    // let transcription_manager = Arc::new(TranscriptionManager::new());
    let manager = Arc::new(Mutex::new(KeyBindingManager::new(
        recording_manager.clone(),
        transcription_manager.clone(),
    )));

    // Register your key bindings
    {
        let mut manager = manager.lock().unwrap();

        // Example: Register Ctrl+Shift combination
        manager.register(
            "ctrl-meta".to_string(),
            vec![Key::ControlRight, Key::MetaRight],
            |ctx| {
                println!("Ctrl+Meta pressed!");
                ctx.recording_manager.try_start_recording("ctrl-meta");
            },
            |ctx| {
                println!("Ctrl+Meta released!");
                if let Some(samples) = ctx.recording_manager.stop_recording("ctrl-meta") {
                    let samples: Vec<f32> = samples; // explicit type annotation
                    match ctx.transcription_manager.transcribe(samples) {
                        Ok(transcription) => println!("Transcription: {}", transcription),
                        Err(err) => println!("Transcription error: {}", err),
                    }
                } else {
                    println!("No samples recorded");
                }
            },
        );
    }

    tauri::async_runtime::spawn({
        let manager = manager.clone();
        async move {
            rdev::listen(move |event| {
                if let Ok(manager) = manager.lock() {
                    manager.handle_event(&event);
                }
            })
            .unwrap();
        }
    });

    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec!["--auto-launch"]),
        ))
        .plugin(tauri_plugin_macos_permissions::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
