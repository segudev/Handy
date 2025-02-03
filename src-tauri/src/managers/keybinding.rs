// First, let's define some types for our audio recording functionality
use super::audio::AudioRecordingManager;
use super::transcription::TranscriptionManager;
use rdev::EventType;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

type KeySet = HashSet<rdev::Key>;

// Now let's modify the KeyBinding structure to work with our new system
#[derive(Clone)]
pub struct BindingContext {
    pub recording_manager: Arc<AudioRecordingManager>,
    pub transcription_manager: Arc<TranscriptionManager>,
    // Add other managers here as needed
    // pub transcription_manager: Arc<TranscriptionManager>,
    // pub llm_manager: Arc<LLMManager>,
}

pub struct KeyBinding {
    id: String,
    keys: KeySet,
    on_press: Box<dyn Fn(&BindingContext) + Send + 'static>,
    on_release: Box<dyn Fn(&BindingContext) + Send + 'static>,
    currently_pressed: Arc<Mutex<KeySet>>,
}

impl KeyBinding {
    fn new<F, G>(id: String, keys: Vec<rdev::Key>, on_press: F, on_release: G) -> Self
    where
        F: Fn(&BindingContext) + Send + 'static,
        G: Fn(&BindingContext) + Send + 'static,
    {
        Self {
            id,
            keys: keys.into_iter().collect(),
            on_press: Box::new(on_press),
            on_release: Box::new(on_release),
            currently_pressed: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    fn handle_event(&self, key: rdev::Key, is_press: bool, context: &BindingContext) -> bool {
        if !self.keys.contains(&key) {
            return false;
        }

        let mut pressed = self.currently_pressed.lock().unwrap();
        if is_press {
            pressed.insert(key);
            if pressed.len() == self.keys.len() && pressed.is_subset(&self.keys) {
                (self.on_press)(context);
                return true;
            }
        } else {
            pressed.remove(&key);
            if pressed.len() == self.keys.len() - 1 {
                (self.on_release)(context);
                return true;
            }
        }
        false
    }
}

pub struct KeyBindingManager {
    bindings: Vec<KeyBinding>,
    context: BindingContext,
}

impl KeyBindingManager {
    pub fn new(
        recording_manager: Arc<AudioRecordingManager>,
        transcription_manager: Arc<TranscriptionManager>,
    ) -> Self {
        Self {
            bindings: Vec::new(),
            context: BindingContext {
                recording_manager,
                transcription_manager,
            },
        }
    }

    pub fn register<F, G>(&mut self, id: String, keys: Vec<rdev::Key>, on_press: F, on_release: G)
    where
        F: Fn(&BindingContext) + Send + 'static,
        G: Fn(&BindingContext) + Send + 'static,
    {
        self.bindings
            .push(KeyBinding::new(id, keys, on_press, on_release));
    }

    pub fn handle_event(&self, event: &rdev::Event) {
        if let EventType::KeyPress(key) | EventType::KeyRelease(key) = event.event_type {
            let is_press = matches!(event.event_type, EventType::KeyPress(_));
            for binding in &self.bindings {
                binding.handle_event(key, is_press, &self.context);
            }
        }
    }
}
