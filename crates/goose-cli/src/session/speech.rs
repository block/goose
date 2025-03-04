use anyhow::Result;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use vosk::{Model, Recognizer};

pub struct SpeechRecognizer {
    model: Model,
    audio_receiver: Option<Receiver<Vec<f32>>>,
    stop_sender: Option<Sender<()>>,
    is_listening: Arc<Mutex<bool>>,
}

impl SpeechRecognizer {
    pub fn new() -> Result<Self> {
        // Initialize Vosk model (download if not present)
        let model = Model::new("model")?;
        
        Ok(Self {
            model,
            audio_receiver: None,
            stop_sender: None,
            is_listening: Arc::new(Mutex::new(false)),
        })
    }

    pub fn start_listening(&mut self) -> Result<()> {
        let (audio_sender, audio_receiver) = channel();
        let (stop_sender, stop_receiver) = channel();
        
        let host = cpal::default_host();
        let device = host.default_input_device()
            .ok_or_else(|| anyhow::anyhow!("No input device found"))?;

        let config = device.default_input_config()?;
        let sample_rate = config.sample_rate().0;
        
        let recognizer = Recognizer::new(&self.model, sample_rate as f32)?;
        let is_listening = Arc::clone(&self.is_listening);
        
        *is_listening.lock().unwrap() = true;

        let stream = device.build_input_stream(
            &config.into(),
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                if stop_receiver.try_recv().is_ok() {
                    return;
                }
                
                // Send audio data for processing
                if let Err(_) = audio_sender.send(data.to_vec()) {
                    return;
                }
            },
            move |err| {
                eprintln!("Error in audio stream: {}", err);
            },
            None,
        )?;

        stream.play()?;

        self.audio_receiver = Some(audio_receiver);
        self.stop_sender = Some(stop_sender);

        Ok(())
    }

    pub fn stop_listening(&mut self) {
        if let Some(sender) = self.stop_sender.take() {
            let _ = sender.send(());
        }
        *self.is_listening.lock().unwrap() = false;
    }

    pub fn is_listening(&self) -> bool {
        *self.is_listening.lock().unwrap()
    }

    pub fn get_text(&self) -> Result<Option<String>> {
        if let Some(receiver) = &self.audio_receiver {
            if let Ok(audio_data) = receiver.try_recv() {
                // Process audio data and return recognized text
                // This is a simplified version - in practice you'd want to
                // accumulate audio until silence is detected
                let recognizer = Recognizer::new(&self.model, 16000.0)?;
                recognizer.accept_waveform(&audio_data);
                if let Some(result) = recognizer.final_result().text() {
                    if !result.is_empty() {
                        return Ok(Some(result.to_string()));
                    }
                }
            }
        }
        Ok(None)
    }
}
