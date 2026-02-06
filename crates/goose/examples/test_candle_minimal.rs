// Minimal test to see if candle works the same in goose repo as in candle repo
use anyhow::Result;
use candle_core::{Device, Tensor};
use candle_transformers::models::quantized_llama::ModelWeights;
use candle_core::quantized::gguf_file;
use tokenizers::Tokenizer;

fn main() -> Result<()> {
    let home = std::env::var("HOME").map_err(anyhow::Error::msg)?;
    let model_path = std::path::PathBuf::from(format!("{}/.local/share/goose/models/llama-3.2-3b.gguf", home));
    let tokenizer_path = std::path::PathBuf::from(format!("{}/.local/share/goose/models/llama-3.2-3b_tokenizer.json", home));

    let prompt = std::fs::read_to_string("/tmp/goose_prompt_stream.txt")?;

    // Device
    let device = if let Ok(device) = Device::new_metal(0) {
        device
    } else {
        Device::Cpu
    };

    // Load model
    let mut file = std::fs::File::open(&model_path)?;
    let content = gguf_file::Content::read(&mut file)?;
    let mut model = ModelWeights::from_gguf(content, &mut file, &device)?;

    // Load tokenizer
    let tokenizer = Tokenizer::from_file(tokenizer_path).map_err(anyhow::Error::msg)?;

    // Tokenize - use false since prompt already has <|begin_of_text|>
    let tokens = tokenizer.encode(prompt.as_str(), false).map_err(anyhow::Error::msg)?;
    let prompt_tokens = tokens.get_ids().to_vec();

    println!("Prompt tokens: {}", prompt_tokens.len());

    // Split-prompt prefill
    let mut next_token = 0u32;
    for (pos, &token) in prompt_tokens.iter().enumerate() {
        let input = Tensor::new(&[token], &device)?.unsqueeze(0)?;
        let logits = model.forward(&input, pos)?;
        let logits = logits.squeeze(0)?;
        next_token = logits.argmax(0)?.to_scalar::<u32>()?;

        if pos >= prompt_tokens.len().saturating_sub(5) {
            println!("pos={}, input_token={}, next_token={}", pos, token, next_token);
        }
    }

    // Decode first token
    let decoded = tokenizer.decode(&[next_token], false).map_err(anyhow::Error::msg)?;
    println!("\nFirst generated token: ID={}, text='{}'", next_token, decoded);

    Ok(())
}
