use std::sync::Arc;

use tauri::App;
use tauri::AppHandle;
use tauri::Manager;
use tauri_plugin_global_shortcut::GlobalShortcutExt;
use tauri_plugin_global_shortcut::{Shortcut, ShortcutState};

use crate::managers::audio::AudioRecordingManager;
use crate::managers::transcription::TranscriptionManager;
use crate::settings;
use crate::settings::ShortcutBinding;
use crate::utils;

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
    let settings = settings::load_or_create_app_settings(app);

    // Register shortcuts with the bindings from settings
    for (_id, binding) in settings.bindings {
        _register_shortcut(app.handle(), binding);
    }
}

#[tauri::command]
pub fn change_binding(app: AppHandle, id: String, binding: String) -> ShortcutBinding {
    let mut settings = settings::get_settings(&app);

    // Get the binding to modify
    let binding_to_modify = settings.bindings.get(&id).unwrap().clone();

    // Unregister the existing binding
    _unregister_shortcut(&app, binding_to_modify.clone());

    // Create an updated binding
    let mut updated_binding = binding_to_modify;
    updated_binding.current_binding = binding;

    // Register the new binding
    _register_shortcut(&app, updated_binding.clone());

    // Update the binding in the settings
    settings.bindings.insert(id, updated_binding.clone());

    // Save the settings
    settings::write_settings(&app, settings);

    // Return the updated binding
    updated_binding
}

#[tauri::command]
pub fn reset_binding(app: AppHandle, id: String) -> ShortcutBinding {
    let binding = settings::get_stored_binding(&app, &id);
    return change_binding(app, id, binding.default_binding);
}

fn _register_shortcut(app: &AppHandle, binding: ShortcutBinding) {
    let shortcut = binding.current_binding.parse::<Shortcut>().unwrap();

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
}

fn _unregister_shortcut(app: &AppHandle, binding: ShortcutBinding) {
    let shortcut = binding.current_binding.parse::<Shortcut>().unwrap();

    app.global_shortcut()
        .unregister(shortcut)
        .expect("couldnt unregister shortcut");
}
