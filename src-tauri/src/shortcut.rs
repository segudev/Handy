use std::sync::Arc;

use tauri::App;
use tauri::AppHandle;
use tauri::Manager;
use tauri::Runtime;
use tauri_plugin_global_shortcut::GlobalShortcutExt;
use tauri_plugin_global_shortcut::{Shortcut, ShortcutState};
use tauri_plugin_store::StoreExt;

use crate::managers::audio::AudioRecordingManager;
use crate::managers::transcription::TranscriptionManager;
use crate::settings;
use crate::settings::AppSettings;
use crate::settings::ShortcutBinding;
use crate::utils;
use crate::AppState;

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
                    utils::paste(transcription, ah);
                }
                Err(err) => println!("Global Shortcut Transcription error: {}", err),
            }
        }
    });
}

pub fn init_shortcuts(app: &App) {
    // Initialize store
    let kb_store = app
        .store("settings_store.json")
        .expect("Failed to initialize store");

    // Get or create settings
    let settings = if let Some(settings_value) = kb_store.get("settings") {
        // Parse the entire settings object
        match serde_json::from_value::<AppSettings>(settings_value) {
            Ok(settings) => {
                println!("Found existing settings: {:?}", settings);

                settings
            }
            Err(e) => {
                println!("Failed to parse settings: {}", e);
                // Fall back to default settings if parsing fails
                let default_settings = settings::get_default_settings();

                // Store the default settings
                kb_store.set("settings", serde_json::to_value(&default_settings).unwrap());

                default_settings
            }
        }
    } else {
        // Create default settings
        let default_settings = settings::get_default_settings();

        // Store the settings
        kb_store.set("settings", serde_json::to_value(&default_settings).unwrap());

        default_settings
    };

    // Register shortcuts with the bindings from settings
    _register_shortcuts(app, settings.bindings);
}

#[tauri::command]
pub fn set_binding(app: AppHandle, id: String, binding: String) {
    let app_state = app.state::<AppState>();
    let mut active_bindings = app_state.active_bindings.lock().unwrap();
    let old_binding_str = active_bindings.get(&id).cloned(); // Clone to avoid borrowing issues

    match binding.parse::<Shortcut>() {
        Ok(shortcut) => {
            println!("Shortcut '{}' is valid", shortcut);
            // unregister the existing binding
            if let Some(old_binding_str) = old_binding_str {
                match old_binding_str.parse::<Shortcut>() {
                    Ok(old_binding) => {
                        app.global_shortcut()
                            .unregister(old_binding)
                            .expect("Failed to unregister shortcut");
                    }
                    Err(e) => {
                        eprintln!("Error parsing old shortcut '{}': {:?}", old_binding_str, e);
                    }
                }
            } else {
                println!("No existing shortcut to unregister");
            }

            // register the new binding
            app.global_shortcut()
                .on_shortcut(shortcut, move |handler_app, scut, event| {
                    if scut == &shortcut {
                        println!("Global Shortcut pressed! {}", scut.into_string());
                        if event.state == ShortcutState::Pressed {
                            transcribe_pressed(handler_app);
                        } else if event.state == ShortcutState::Released {
                            transcribe_released(handler_app);
                        }
                    }
                })
                .expect("couldnt register shortcut");

            // TODO error handling?
            active_bindings.insert(id, binding);
        }
        Err(e) => {
            eprintln!("Error parsing shortcut '{}': {:?}", binding, e);
        }
    }
}

fn _register_shortcuts(app: &App, bindings: Vec<ShortcutBinding>) {
    // get bindings from state
    let app_state = app.state::<AppState>();
    let mut active_bindings = app_state.active_bindings.lock().unwrap();

    // iterate through bindings
    for binding in bindings {
        // Parse the shortcut, handling errors gracefully
        match binding.current_binding.parse::<Shortcut>() {
            Ok(shortcut) => {
                // Try to register the shortcut
                match app.global_shortcut().on_shortcut(
                    shortcut,
                    move |handler_app, scut, event| {
                        if scut == &shortcut {
                            println!("Global Shortcut pressed! {}", scut.into_string());
                            if event.state == ShortcutState::Pressed {
                                transcribe_pressed(handler_app);
                            } else if event.state == ShortcutState::Released {
                                transcribe_released(handler_app);
                            }
                        }
                    },
                ) {
                    Ok(_) => {
                        // Additional actions on successful registration
                        println!(
                            "Successfully registered shortcut: {}",
                            binding.current_binding
                        );

                        active_bindings.insert(binding.id, binding.current_binding);
                    }
                    Err(err) => {
                        // Log registration error
                        eprintln!(
                            "Failed to register shortcut {}: {}",
                            binding.current_binding, err
                        );
                    }
                }
            }
            Err(err) => {
                // Log parsing error
                eprintln!(
                    "Failed to parse shortcut {}: {}",
                    binding.current_binding, err
                );
            }
        }
    }
}
