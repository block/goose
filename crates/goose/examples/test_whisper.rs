use goose::dictation::whisper::{get_model, WhisperTranscriber};

const WHISPER_TOKENIZER_JSON: &str = include_str!("../src/dictation/whisper_data/tokens.json");

fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    let audio_path = "/tmp/whisper_audio_16k.wav";
    let model_id = "tiny";

    let model =
        get_model(model_id).ok_or_else(|| anyhow::anyhow!("Model {} not found", model_id))?;
    let model_path = model.local_path();

    println!("Loading model from: {}", model_path.display());
    let mut transcriber =
        WhisperTranscriber::new_with_tokenizer(model_id, &model_path, WHISPER_TOKENIZER_JSON)?;

    println!("Reading audio from: {}", audio_path);
    let audio_data = std::fs::read(audio_path)?;

    println!("Transcribing...");
    let text = transcriber.transcribe(&audio_data)?;

    println!("\n========== FINAL TRANSCRIPTION ==========");
    println!("{}", text);
    println!("=========================================\n");

    Ok(())
}
