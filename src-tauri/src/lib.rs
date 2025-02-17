mod managers;

use log::{info, Level};
use managers::keybinding::KeyBindingManager;
use managers::transcription::TranscriptionManager;
use managers::{audio::AudioRecordingManager, transcription};
use rdev::{simulate, EventType, Key, SimulateError};
use rig::{completion::Prompt, providers::anthropic};
use std::sync::{Arc, Mutex};
use std::{thread, time};
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_clipboard_manager::ClipboardExt;

fn try_send_event(event: &EventType) {
    if let Err(SimulateError) = simulate(event) {
        println!("We could not send {:?}", event);
    }
}

fn send(event: EventType) {
    try_send_event(&event);
    thread::sleep(time::Duration::from_millis(20));
}

fn send_multiple(events: &[EventType]) {
    events.iter().for_each(try_send_event);
    thread::sleep(time::Duration::from_millis(20));
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

    // Release both keys simultaneously
    send_multiple(&[
        EventType::KeyRelease(Key::KeyV),
        EventType::KeyRelease(modifier_key),
    ]);

    // Additional delay after the complete combination
    // thread::sleep(time::Duration::from_millis(50));
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

fn get_highlighted_text(app_handle: tauri::AppHandle) -> String {
    let clipboard = app_handle.clipboard();

    // save the clipboard content
    let clipboard_content = clipboard.read_text().unwrap_or_default();

    // empty the clipboard
    clipboard.write_text("").unwrap();

    // get the highlighted text
    let highlighted_text = clipboard.read_text().unwrap_or_default();

    // restore the clipboard content
    clipboard.write_text(&clipboard_content).unwrap();

    highlighted_text
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

    // Register LLM Call after Transcription
    manager.register(
        "shift-alt".to_string(),
        vec![Key::ShiftLeft, Key::Alt],
        |ctx| {
            info!("Shift+Alt pressed!");
            ctx.recording_manager.try_start_recording("shift-alt");
            None
        },
        |ctx| {
            info!("release being called from shift-alt");
            let ctx = ctx.clone();
            Some(tauri::async_runtime::spawn(async move {
                if let Some(samples) = ctx.recording_manager.stop_recording("shift-alt") {
                    if let Ok(transcription) = ctx.transcription_manager.transcribe(samples) {
                        println!("Transcription: {}", transcription);
                        match ctx.sonnet.prompt(transcription).await {
                            Ok(response) => {
                                let highlighted_text = get_highlighted_text(ctx.app_handle.clone());
                                println!("Highlighted Text: {}", highlighted_text);
                                println!("Sonnet response: {}", response);
                                paste(response, ctx.app_handle.clone());
                            }
                            Err(err) => println!("Sonnet error: {}", err),
                        }
                    }
                }
            }))
        },
    );

    manager.register(
        "ctrl-alt-meta".to_string(),
        vec![Key::ControlLeft, Key::Alt, Key::MetaLeft],
        |ctx| {
            info!("Ctrl+Alt+Meta pressed!");
            ctx.recording_manager.try_start_recording("ctrl-alt-meta");
            None
        },
        |ctx| {
            info!("release being called from ctrl-alt-meta");
            let ctx = ctx.clone();
            Some(tauri::async_runtime::spawn(async move {
                if let Some(samples) = ctx.recording_manager.stop_recording("ctrl-alt-meta") {
                    let samples: Vec<f32> = samples; // explicit type annotation
                    match ctx.transcription_manager.transcribe(samples) {
                        Ok(transcription) => {
                            println!("Transcription: {}", transcription);
                            // Call LLM for code
                        }
                        Err(err) => println!("Transcription error: {}", err),
                    }
                } else {
                    println!("No samples recorded");
                }
            }))
        },
    );
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::init();

    let recording_manager =
        Arc::new(AudioRecordingManager::new().expect("Failed to initialize recording manager"));
    let transcription_manager =
        Arc::new(TranscriptionManager::new().expect("Failed to initialize transcription manager"));
    // let transcription_manager = Arc::new(TranscriptionManager::new());
    let claude_client = anthropic::Client::from_env();
    let sonnet: Arc<rig::agent::Agent<anthropic::completion::CompletionModel>> = Arc::new(
        claude_client
            .agent(anthropic::CLAUDE_3_5_SONNET)
            .preamble("Be precise and concise.")
            .temperature(0.5)
            .build(),
    );

    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec!["--auto-launch"]),
        ))
        .plugin(tauri_plugin_macos_permissions::init())
        .plugin(tauri_plugin_opener::init())
        .setup(move |app| {
            let app_handle = app.handle().clone();

            let manager = Arc::new(Mutex::new(KeyBindingManager::new(
                recording_manager.clone(),
                transcription_manager.clone(),
                sonnet.clone(),
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
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
