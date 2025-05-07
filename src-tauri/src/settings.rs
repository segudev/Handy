use serde::Deserialize;
use serde::Serialize;

#[derive(Serialize, Deserialize, Debug, Clone)] // Clone is useful
pub struct ShortcutBinding {
    pub id: String,
    pub name: String,
    pub description: String,
    pub default_binding: String,
    pub current_binding: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AppSettings {
    pub bindings: Vec<ShortcutBinding>,
}

pub fn get_default_settings() -> AppSettings {
    AppSettings {
        bindings: vec![
            ShortcutBinding {
                id: "transcribe".to_string(),
                name: "Transcribe".to_string(),
                description: "Converts your speech into text.".to_string(),
                default_binding: "alt+space".to_string(),
                current_binding: "alt+space".to_string(),
            },
            ShortcutBinding {
                id: "test".to_string(),
                name: "Test".to_string(),
                description: "This is a test binding.".to_string(),
                default_binding: "ctrl+d".to_string(),
                current_binding: "ctrl+d".to_string(),
            },
        ],
    }
}
