use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};
use std::vec::Vec;

use samplerate::{convert, ConverterType};

#[derive(Clone, Debug)]
pub enum RecordingState {
    Initializing,
    Idle,
    Recording { binding_id: String },
}

pub struct AudioRecordingManager {
    state: Arc<Mutex<RecordingState>>,
    buffer: Arc<Mutex<Vec<f32>>>,
    channels: u16,
    sample_rate: u32,
}

impl AudioRecordingManager {
    pub fn new() -> Result<Self, anyhow::Error> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or_else(|| anyhow::Error::msg("No input device available"))?;

        let config = device.default_input_config()?;

        // Log the audio configuration details
        println!("Sample Format: {:?}", config.sample_format());
        println!("Channel Count: {}", config.channels());
        println!("Sample Rate: {} Hz", config.sample_rate().0);
        println!("Buffer Size: {:?}", config.buffer_size());

        let channels = config.channels();
        let sample_rate = config.sample_rate().0;

        let state = Arc::new(Mutex::new(RecordingState::Idle));
        let buffer = Arc::new(Mutex::new(Vec::new()));

        let state_clone = Arc::clone(&state);
        let buffer_clone = Arc::clone(&buffer);

        // Create and start stream in a separate thread
        std::thread::spawn(move || {
            let stream = match config.sample_format() {
                cpal::SampleFormat::F32 => device.build_input_stream(
                    &config.into(),
                    move |data: &[f32], _: &cpal::InputCallbackInfo| {
                        let state_guard = state_clone.lock().unwrap();
                        if let RecordingState::Recording { .. } = *state_guard {
                            let mut buffer = buffer_clone.lock().unwrap();
                            buffer.extend_from_slice(data);
                        }
                    },
                    |err| eprintln!("Error in stream: {}", err),
                    None,
                ),
                sample_format => panic!("Unsupported sample format: {:?}", sample_format),
            }
            .unwrap();

            stream.play().unwrap();

            // Keep the stream alive
            std::thread::park();
        });

        Ok(Self {
            state,
            buffer,
            channels,
            sample_rate,
        })
    }

    pub fn try_start_recording(&self, binding_id: &str) -> bool {
        let mut state = self.state.lock().unwrap();
        match *state {
            RecordingState::Idle => {
                // Clear the buffer before starting new recording
                self.buffer.lock().unwrap().clear();
                *state = RecordingState::Recording {
                    binding_id: binding_id.to_string(),
                };
                println!("Started recording for binding {}", binding_id);
                true
            }
            RecordingState::Recording {
                binding_id: ref active_id,
            } => {
                println!(
                    "Cannot start recording: already recording for binding {}",
                    active_id
                );
                false
            }
            RecordingState::Initializing => {
                println!("Cannot start recording: initializing");
                false
            }
        }
    }

    pub fn stop_recording(&self, binding_id: &str) -> Option<Vec<f32>> {
        let mut state = self.state.lock().unwrap();
        match *state {
            RecordingState::Recording {
                binding_id: ref active_id,
            } if active_id == binding_id => {
                *state = RecordingState::Idle;
                println!("Stopped recording for binding {}", binding_id);

                let mut buffer = self.buffer.lock().unwrap();
                let mut toResample: Vec<f32> = buffer.drain(..).collect();

                let start = std::time::Instant::now();
                let resampled = convert(
                    self.sample_rate,
                    16000,
                    1,
                    ConverterType::SincBestQuality,
                    &toResample,
                )
                .unwrap();
                let duration = start.elapsed();
                println!("Resampling took: {:?}", duration);

                Some(resampled)
            }
            _ => {
                println!("Cannot stop recording: not recording or wrong binding");
                None
            }
        }
    }

    pub fn get_current_recording_state(&self) -> RecordingState {
        self.state.lock().unwrap().clone()
    }
}
