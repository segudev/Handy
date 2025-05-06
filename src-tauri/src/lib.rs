mod managers;

use log::info;
use managers::audio::AudioRecordingManager;
use managers::keybinding::KeyBindingManager;
use managers::transcription::TranscriptionManager;
use rdev::{simulate, EventType, Key, SimulateError};
use std::sync::{Arc, Mutex};
use std::{thread, time};
use tauri::{Emitter, Manager};
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_clipboard_manager::ClipboardExt;

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

fn paste(text: String, app_handle: tauri::AppHandle) {
    let clipboard = app_handle.clipboard();

    // get the current clipboard content
    let clipboard_content = clipboard.read_text().unwrap_or_default();

    clipboard.write_text(&text).unwrap();
    send_paste();

    // restore the clipboard
    clipboard.write_text(&clipboard_content).unwrap();
}

fn register_bindings(manager: &mut KeyBindingManager) {
    manager.register(
        "ctrl-meta".to_string(),
        vec![Key::ControlRight, Key::MetaRight],
        |ctx| {
            info!("Ctrl+Meta pressed!");
            ctx.recording_manager.try_start_recording("ctrl-meta");
            None
        },
        |ctx| {
            info!("release being called from ctrl-meta");
            let ctx = ctx.clone();
            Some(tauri::async_runtime::spawn(async move {
                if let Some(samples) = ctx.recording_manager.stop_recording("ctrl-meta") {
                    match ctx.transcription_manager.transcribe(samples) {
                        Ok(transcription) => {
                            println!("Transcription: {}", transcription);
                            paste(transcription, ctx.app_handle.clone());
                        }
                        Err(err) => println!("Transcription error: {}", err),
                    }
                }
            }))
        },
    );
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_stronghold::Builder::new(|_pass| todo!()).build())
        .plugin(tauri_plugin_upload::init())
        // .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec!["--auto-launch"]),
        ))
        .plugin(tauri_plugin_macos_permissions::init())
        .plugin(tauri_plugin_opener::init())
        .setup(move |app| {
            let app_handle = app.handle().clone();

            let vad_path = app.path().resolve(
                "resources/silero_vad_v4.onnx",
                tauri::path::BaseDirectory::Resource,
            )?;

            let whisper_path = app.path().resolve(
                "resources/ggml-small.bin",
                tauri::path::BaseDirectory::Resource,
            )?;

            let recording_manager = Arc::new(
                AudioRecordingManager::new(&vad_path)
                    .expect("Failed to initialize recording manager"),
            );
            let transcription_manager = Arc::new(
                TranscriptionManager::new(
                    whisper_path
                        .to_str()
                        .expect("Path contains invalid UTF-8 Chars"),
                )
                .expect("Failed to initialize transcription manager"),
            );

            let manager = Arc::new(Mutex::new(KeyBindingManager::new(
                recording_manager.clone(),
                transcription_manager.clone(),
                // claude_client.clone(),
                app_handle.clone(),
            )));

            {
                let mut manager = manager.lock().unwrap();
                register_bindings(&mut manager);
            }

            let manager_clone = manager.clone();
            tauri::async_runtime::spawn(async move {
                rdev::listen(move |event| {
                    if let Ok(manager) = manager_clone.lock() {
                        manager.handle_event(&event);
                    }
                })
                .unwrap();
            });

            #[cfg(desktop)]
            {
                use tauri_plugin_global_shortcut::{Code, Modifiers, ShortcutState};

                app.handle().plugin(
                    tauri_plugin_global_shortcut::Builder::new()
                        .with_shortcuts(["ctrl+d", "alt+space"])?
                        .with_handler(|app, shortcut, event| {
                            if event.state == ShortcutState::Pressed {
                                if shortcut.matches(Modifiers::CONTROL, Code::KeyD) {
                                    let _ = app.emit("shortcut-event", "Ctrl+D triggered");
                                    println!("Ctrl+D triggered");
                                }
                                if shortcut.matches(Modifiers::ALT, Code::Space) {
                                    let _ = app.emit("shortcut-event", "Alt+Space triggered");
                                    println!("Alt+Space triggered");
                                }
                            }
                            if event.state == ShortcutState::Released {
                                if shortcut.matches(Modifiers::CONTROL, Code::KeyD) {
                                    let _ = app.emit("shortcut-event", "Ctrl+D released");
                                    println!("Ctrl+D released");
                                }
                                if shortcut.matches(Modifiers::ALT, Code::Space) {
                                    let _ = app.emit("shortcut-event", "Alt+Space released");
                                    println!("Alt+Space released");
                                }
                            }
                        })
                        .build(),
                )?;
            }

            Ok(())
        })
        .on_window_event(|app, event| match event {
            tauri::WindowEvent::CloseRequested { api, .. } => {
                app.hide().unwrap();
                api.prevent_close();
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
