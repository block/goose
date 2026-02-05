#[cfg(feature = "mkl")]
extern crate intel_mkl_src;

#[cfg(feature = "accelerate")]
extern crate accelerate_src;

use clap::{Parser, ValueEnum};
use std::io::Write;
use tokenizers::Tokenizer;

use candle::quantized::{ggml_file, gguf_file};
use candle::Tensor;
use candle_transformers::generation::{LogitsProcessor, Sampling};

use candle_examples::token_output_stream::TokenOutputStream;
use candle_transformers::models::quantized_llama as model;
use model::ModelWeights;

const DEFAULT_PROMPT: &str = "My favorite theorem is ";

#[derive(Debug)]
enum Prompt {
    Interactive,
    Chat,
    One(String),
}

#[derive(Clone, Debug, Copy, PartialEq, Eq, ValueEnum)]
enum Which {
    #[value(name = "7b")]
    L7b,
    #[value(name = "13b")]
    L13b,
    #[value(name = "70b")]
    L70b,
    #[value(name = "7b-chat")]
    L7bChat,
    #[value(name = "13b-chat")]
    L13bChat,
    #[value(name = "70b-chat")]
    L70bChat,
    #[value(name = "7b-code")]
    L7bCode,
    #[value(name = "13b-code")]
    L13bCode,
    #[value(name = "32b-code")]
    L34bCode,
    #[value(name = "7b-leo")]
    Leo7b,
    #[value(name = "13b-leo")]
    Leo13b,
    #[value(name = "7b-mistral")]
    Mistral7b,
    #[value(name = "7b-mistral-instruct")]
    Mistral7bInstruct,
    #[value(name = "7b-mistral-instruct-v0.2")]
    Mistral7bInstructV02,
    #[value(name = "7b-zephyr-a")]
    Zephyr7bAlpha,
    #[value(name = "7b-zephyr-b")]
    Zephyr7bBeta,
    #[value(name = "7b-open-chat-3.5")]
    OpenChat35,
    #[value(name = "7b-starling-a")]
    Starling7bAlpha,
    #[value(name = "mixtral")]
    Mixtral,
    #[value(name = "mixtral-instruct")]
    MixtralInstruct,
    #[value(name = "llama3-8b")]
    L8b,
    #[value(name = "phi3")]
    Phi3,
    #[value(name = "SmoLM2-360M-Instruct")]
    SmolLM2_360MInstruct,
    #[value(name = "SmoLM2-1.7B-Instruct")]
    SmolLM2_1BInstruct,
    #[value(name = "deepseekr1-llama8b")]
    DeepseekR1Llama8b,
}

impl Which {
    fn is_mistral(&self) -> bool {
        match self {
            Self::L7b
            | Self::L13b
            | Self::L70b
            | Self::L7bChat
            | Self::L13bChat
            | Self::L70bChat
            | Self::L7bCode
            | Self::L13bCode
            | Self::L34bCode
            | Self::Leo7b
            | Self::Leo13b
            | Self::L8b
            | Self::Phi3
            | Self::SmolLM2_1BInstruct
            | Self::SmolLM2_360MInstruct
            | Self::DeepseekR1Llama8b => false,
            // Zephyr and OpenChat are fine tuned versions of mistral and should be treated in the
            // same way. Starling is a fine tuned version of OpenChat.
            Self::OpenChat35
            | Self::Starling7bAlpha
            | Self::Zephyr7bAlpha
            | Self::Zephyr7bBeta
            | Self::Mixtral
            | Self::MixtralInstruct
            | Self::Mistral7b
            | Self::Mistral7bInstruct
            | Self::Mistral7bInstructV02 => true,
        }
    }

    fn is_zephyr(&self) -> bool {
        match self {
            Self::L7b
            | Self::L13b
            | Self::L70b
            | Self::L7bChat
            | Self::L13bChat
            | Self::L70bChat
            | Self::L7bCode
            | Self::L13bCode
            | Self::L34bCode
            | Self::Leo7b
            | Self::Leo13b
            | Self::Mixtral
            | Self::MixtralInstruct
            | Self::Mistral7b
            | Self::Mistral7bInstruct
            | Self::Mistral7bInstructV02
            | Self::OpenChat35
            | Self::Starling7bAlpha
            | Self::L8b
            | Self::SmolLM2_1BInstruct
            | Self::SmolLM2_360MInstruct
            | Self::Phi3
            | Self::DeepseekR1Llama8b => false,
            Self::Zephyr7bAlpha | Self::Zephyr7bBeta => true,
        }
    }

    fn is_open_chat(&self) -> bool {
        match self {
            Self::L7b
            | Self::L13b
            | Self::L70b
            | Self::L7bChat
            | Self::L13bChat
            | Self::L70bChat
            | Self::L7bCode
            | Self::L13bCode
            | Self::L34bCode
            | Self::Leo7b
            | Self::Leo13b
            | Self::Mixtral
            | Self::MixtralInstruct
            | Self::Mistral7b
            | Self::Mistral7bInstruct
            | Self::Mistral7bInstructV02
            | Self::Zephyr7bAlpha
            | Self::Zephyr7bBeta
            | Self::L8b
            | Self::SmolLM2_1BInstruct
            | Self::SmolLM2_360MInstruct
            | Self::Phi3
            | Self::DeepseekR1Llama8b => false,
            Self::OpenChat35 | Self::Starling7bAlpha => true,
        }
    }

    fn is_deepseek(&self) -> bool {
        match self {
            Self::L7b
            | Self::L13b
            | Self::L70b
            | Self::L7bChat
            | Self::L13bChat
            | Self::L70bChat
            | Self::L7bCode
            | Self::L13bCode
            | Self::L34bCode
            | Self::Leo7b
            | Self::Leo13b
            | Self::Mixtral
            | Self::MixtralInstruct
            | Self::Mistral7b
            | Self::Mistral7bInstruct
            | Self::Mistral7bInstructV02
            | Self::Zephyr7bAlpha
            | Self::Zephyr7bBeta
            | Self::L8b
            | Self::SmolLM2_1BInstruct
            | Self::SmolLM2_360MInstruct
            | Self::Phi3
            | Self::OpenChat35
            | Self::Starling7bAlpha => false,
            Self::DeepseekR1Llama8b => true,
        }
    }
    fn tokenizer_repo(&self) -> &'static str {
        match self {
            Self::L7b
            | Self::L13b
            | Self::L70b
            | Self::L7bChat
            | Self::L13bChat
            | Self::L70bChat
            | Self::L7bCode
            | Self::L13bCode
            | Self::L34bCode => "hf-internal-testing/llama-tokenizer",
            Self::Leo7b => "LeoLM/leo-hessianai-7b",
            Self::Leo13b => "LeoLM/leo-hessianai-13b",
            Self::Mixtral => "mistralai/Mixtral-8x7B-v0.1",
            Self::MixtralInstruct => "mistralai/Mixtral-8x7B-Instruct-v0.1",
            Self::Mistral7b
            | Self::Mistral7bInstruct
            | Self::Mistral7bInstructV02
            | Self::Zephyr7bAlpha
            | Self::Zephyr7bBeta => "mistralai/Mistral-7B-v0.1",
            Self::OpenChat35 => "openchat/openchat_3.5",
            Self::Starling7bAlpha => "berkeley-nest/Starling-LM-7B-alpha",
            Self::L8b => "meta-llama/Meta-Llama-3-8B",
            Self::Phi3 => "microsoft/Phi-3-mini-4k-instruct",
            Self::SmolLM2_360MInstruct => "HuggingFaceTB/SmolLM2-360M-Instruct",
            Self::SmolLM2_1BInstruct => "HuggingFaceTB/SmolLM2-1.7B-Instruct",
            Self::DeepseekR1Llama8b => "deepseek-ai/DeepSeek-R1-Distill-Llama-8B",
        }
    }
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// GGML/GGUF file to load, typically a .bin/.gguf file generated by the quantize command from llama.cpp
    #[arg(long)]
    model: Option<String>,

    /// The initial prompt, use 'interactive' for entering multiple prompts in an interactive way
    /// and 'chat' for an interactive model where history of previous prompts and generated tokens
    /// is preserved.
    #[arg(long)]
    prompt: Option<String>,

    /// The length of the sample to generate (in tokens).
    #[arg(short = 'n', long, default_value_t = 1000)]
    sample_len: usize,

    /// The tokenizer config in json format.
    #[arg(long)]
    tokenizer: Option<String>,

    /// The temperature used to generate samples, use 0 for greedy sampling.
    #[arg(long, default_value_t = 0.8)]
    temperature: f64,

    /// Nucleus sampling probability cutoff.
    #[arg(long)]
    top_p: Option<f64>,

    /// Only sample among the top K samples.
    #[arg(long)]
    top_k: Option<usize>,

    /// The seed to use when generating random samples.
    #[arg(long, default_value_t = 299792458)]
    seed: u64,

    /// Enable tracing (generates a trace-timestamp.json file).
    #[arg(long)]
    tracing: bool,

    /// Display the token for the specified prompt.
    #[arg(long)]
    verbose_prompt: bool,

    /// Process prompt elements separately.
    #[arg(long)]
    split_prompt: bool,

    /// Run on CPU rather than GPU even if a GPU is available.
    #[arg(long)]
    cpu: bool,

    /// Penalty to be applied for repeating tokens, 1. means no penalty.
    #[arg(long, default_value_t = 1.1)]
    repeat_penalty: f32,

    /// The context size to consider for the repeat penalty.
    #[arg(long, default_value_t = 64)]
    repeat_last_n: usize,

    /// The model size to use.
    #[arg(long, default_value = "7b")]
    which: Which,

    /// Group-Query Attention, use 8 for the 70B version of LLaMAv2.
    #[arg(long)]
    gqa: Option<usize>,

    /// Use the slower dmmv cuda kernel.
    #[arg(long)]
    force_dmmv: bool,
}

impl Args {
    fn tokenizer(&self) -> anyhow::Result<Tokenizer> {
        let tokenizer_path = match &self.tokenizer {
            Some(config) => std::path::PathBuf::from(config),
            None => {
                let api = hf_hub::api::sync::Api::new()?;
                let repo = self.which.tokenizer_repo();
                let api = api.model(repo.to_string());
                api.get("tokenizer.json")?
            }
        };
        Tokenizer::from_file(tokenizer_path).map_err(anyhow::Error::msg)
    }

    fn model(&self) -> anyhow::Result<std::path::PathBuf> {
        let model_path = match &self.model {
            Some(config) => std::path::PathBuf::from(config),
            None => {
                let (repo, filename) = match self.which {
                    Which::L7b => ("TheBloke/Llama-2-7B-GGML", "llama-2-7b.ggmlv3.q4_0.bin"),
                    Which::L13b => ("TheBloke/Llama-2-13B-GGML", "llama-2-13b.ggmlv3.q4_0.bin"),
                    Which::L70b => ("TheBloke/Llama-2-70B-GGML", "llama-2-70b.ggmlv3.q4_0.bin"),
                    Which::L7bChat => (
                        "TheBloke/Llama-2-7B-Chat-GGML",
                        "llama-2-7b-chat.ggmlv3.q4_0.bin",
                    ),
                    Which::L13bChat => (
                        "TheBloke/Llama-2-13B-Chat-GGML",
                        "llama-2-13b-chat.ggmlv3.q4_0.bin",
                    ),
                    Which::L70bChat => (
                        "TheBloke/Llama-2-70B-Chat-GGML",
                        "llama-2-70b-chat.ggmlv3.q4_0.bin",
                    ),
                    Which::L7bCode => ("TheBloke/CodeLlama-7B-GGUF", "codellama-7b.Q8_0.gguf"),
                    Which::L13bCode => ("TheBloke/CodeLlama-13B-GGUF", "codellama-13b.Q8_0.gguf"),
                    Which::L34bCode => ("TheBloke/CodeLlama-34B-GGUF", "codellama-34b.Q8_0.gguf"),
                    Which::Leo7b => (
                        "TheBloke/leo-hessianai-7B-GGUF",
                        "leo-hessianai-7b.Q4_K_M.gguf",
                    ),
                    Which::Leo13b => (
                        "TheBloke/leo-hessianai-13B-GGUF",
                        "leo-hessianai-13b.Q4_K_M.gguf",
                    ),
                    Which::Mixtral => (
                        "TheBloke/Mixtral-8x7B-v0.1-GGUF",
                        "mixtral-8x7b-v0.1.Q4_K_M.gguf",
                    ),
                    Which::MixtralInstruct => (
                        "TheBloke/Mixtral-8x7B-Instruct-v0.1-GGUF",
                        "mixtral-8x7b-instruct-v0.1.Q4_K_M.gguf",
                    ),
                    Which::Mistral7b => (
                        "TheBloke/Mistral-7B-v0.1-GGUF",
                        "mistral-7b-v0.1.Q4_K_S.gguf",
                    ),
                    Which::Mistral7bInstruct => (
                        "TheBloke/Mistral-7B-Instruct-v0.1-GGUF",
                        "mistral-7b-instruct-v0.1.Q4_K_S.gguf",
                    ),
                    Which::Mistral7bInstructV02 => (
                        "TheBloke/Mistral-7B-Instruct-v0.2-GGUF",
                        "mistral-7b-instruct-v0.2.Q4_K_S.gguf",
                    ),
                    Which::Zephyr7bAlpha => (
                        "TheBloke/zephyr-7B-alpha-GGUF",
                        "zephyr-7b-alpha.Q4_K_M.gguf",
                    ),
                    Which::Zephyr7bBeta => {
                        ("TheBloke/zephyr-7B-beta-GGUF", "zephyr-7b-beta.Q4_K_M.gguf")
                    }
                    Which::OpenChat35 => ("TheBloke/openchat_3.5-GGUF", "openchat_3.5.Q4_K_M.gguf"),
                    Which::Starling7bAlpha => (
                        "TheBloke/Starling-LM-7B-alpha-GGUF",
                        "starling-lm-7b-alpha.Q4_K_M.gguf",
                    ),
                    // TODO: swap to TheBloke model when available
                    Which::L8b => (
                        "QuantFactory/Meta-Llama-3-8B-GGUF",
                        "Meta-Llama-3-8B.Q4_K_S.gguf",
                    ),
                    Which::Phi3 => (
                        "microsoft/Phi-3-mini-4k-instruct-gguf",
                        "Phi-3-mini-4k-instruct-q4.gguf",
                    ),
                    Which::SmolLM2_360MInstruct => (
                        "HuggingFaceTB/SmolLM2-360M-Instruct-GGUF",
                        "smollm2-360m-instruct-q8_0.gguf",
                    ),
                    Which::SmolLM2_1BInstruct => (
                        "HuggingFaceTB/SmolLM2-1.7B-Instruct-GGUF",
                        "smollm2-1.7b-instruct-q4_k_m.gguf",
                    ),
                    Which::DeepseekR1Llama8b => (
                        "unsloth/DeepSeek-R1-Distill-Llama-8B-GGUF",
                        "DeepSeek-R1-Distill-Llama-8B-Q4_K_M.gguf",
                    ),
                };
                let revision = if self.which == Which::Phi3 {
                    "5eef2ce24766d31909c0b269fe90c817a8f263fb"
                } else {
                    "main"
                };
                let api = hf_hub::api::sync::Api::new()?;
                api.repo(hf_hub::Repo::with_revision(
                    repo.to_string(),
                    hf_hub::RepoType::Model,
                    revision.to_string(),
                ))
                .get(filename)?
            }
        };
        Ok(model_path)
    }
}

fn format_size(size_in_bytes: usize) -> String {
    if size_in_bytes < 1_000 {
        format!("{size_in_bytes}B")
    } else if size_in_bytes < 1_000_000 {
        format!("{:.2}KB", size_in_bytes as f64 / 1e3)
    } else if size_in_bytes < 1_000_000_000 {
        format!("{:.2}MB", size_in_bytes as f64 / 1e6)
    } else {
        format!("{:.2}GB", size_in_bytes as f64 / 1e9)
    }
}

fn main() -> anyhow::Result<()> {
    use tracing_chrome::ChromeLayerBuilder;
    use tracing_subscriber::prelude::*;

    let args = Args::parse();

    #[cfg(feature = "cuda")]
    candle::quantized::cuda::set_force_dmmv(args.force_dmmv);

    candle::cuda::set_gemm_reduced_precision_f16(true);
    candle::cuda::set_gemm_reduced_precision_bf16(true);

    let _guard = if args.tracing {
        let (chrome_layer, guard) = ChromeLayerBuilder::new().build();
        tracing_subscriber::registry().with(chrome_layer).init();
        Some(guard)
    } else {
        None
    };

    println!(
        "avx: {}, neon: {}, simd128: {}, f16c: {}",
        candle::utils::with_avx(),
        candle::utils::with_neon(),
        candle::utils::with_simd128(),
        candle::utils::with_f16c()
    );
    println!(
        "temp: {:.2} repeat-penalty: {:.2} repeat-last-n: {}",
        args.temperature, args.repeat_penalty, args.repeat_last_n
    );

    let model_path = args.model()?;
    let mut file = std::fs::File::open(&model_path)?;
    let start = std::time::Instant::now();
    let device = candle_examples::device(args.cpu)?;

    let mut model = match model_path.extension().and_then(|v| v.to_str()) {
        Some("gguf") => {
            let model = gguf_file::Content::read(&mut file).map_err(|e| e.with_path(model_path))?;
            let mut total_size_in_bytes = 0;
            for (_, tensor) in model.tensor_infos.iter() {
                let elem_count = tensor.shape.elem_count();
                total_size_in_bytes +=
                    elem_count * tensor.ggml_dtype.type_size() / tensor.ggml_dtype.block_size();
            }
            println!(
                "loaded {:?} tensors ({}) in {:.2}s",
                model.tensor_infos.len(),
                &format_size(total_size_in_bytes),
                start.elapsed().as_secs_f32(),
            );
            ModelWeights::from_gguf(model, &mut file, &device)?
        }
        Some("ggml" | "bin") | Some(_) | None => {
            let model = ggml_file::Content::read(&mut file, &device)
                .map_err(|e| e.with_path(model_path))?;
            let mut total_size_in_bytes = 0;
            for (_, tensor) in model.tensors.iter() {
                let elem_count = tensor.shape().elem_count();
                total_size_in_bytes +=
                    elem_count * tensor.dtype().type_size() / tensor.dtype().block_size();
            }
            println!(
                "loaded {:?} tensors ({}) in {:.2}s",
                model.tensors.len(),
                &format_size(total_size_in_bytes),
                start.elapsed().as_secs_f32(),
            );
            println!("params: {:?}", model.hparams);
            let default_gqa = match args.which {
                Which::L7b
                | Which::L13b
                | Which::L7bChat
                | Which::L13bChat
                | Which::L7bCode
                | Which::L13bCode
                | Which::L34bCode
                | Which::Leo7b
                | Which::Leo13b
                | Which::L8b
                | Which::SmolLM2_1BInstruct
                | Which::SmolLM2_360MInstruct
                | Which::DeepseekR1Llama8b
                | Which::Phi3 => 1,
                Which::Mixtral
                | Which::MixtralInstruct
                | Which::Mistral7b
                | Which::Mistral7bInstruct
                | Which::Mistral7bInstructV02
                | Which::Zephyr7bAlpha
                | Which::Zephyr7bBeta
                | Which::L70b
                | Which::L70bChat
                | Which::OpenChat35
                | Which::Starling7bAlpha => 8,
            };
            ModelWeights::from_ggml(model, args.gqa.unwrap_or(default_gqa))?
        }
    };
    println!("model built");

    let tokenizer = args.tokenizer()?;
    let mut tos = TokenOutputStream::new(tokenizer);
    let prompt = match args.prompt.as_deref() {
        Some("chat") => Prompt::Chat,
        Some("interactive") => Prompt::Interactive,
        Some(s) => Prompt::One(s.to_string()),
        None => Prompt::One(DEFAULT_PROMPT.to_string()),
    };

    let mut pre_prompt_tokens = vec![];
    for prompt_index in 0.. {
        let prompt_str = match &prompt {
            Prompt::One(prompt) => prompt.clone(),
            Prompt::Interactive | Prompt::Chat => {
                let is_interactive = matches!(prompt, Prompt::Interactive);
                print!("> ");
                std::io::stdout().flush()?;
                let mut prompt = String::new();
                std::io::stdin().read_line(&mut prompt)?;
                if prompt.ends_with('\n') {
                    prompt.pop();
                    if prompt.ends_with('\r') {
                        prompt.pop();
                    }
                }
                if args.which.is_open_chat() {
                    format!("GPT4 Correct User: {prompt}<|end_of_turn|>GPT4 Correct Assistant:")
                } else if args.which.is_zephyr() {
                    if prompt_index == 0 || is_interactive {
                        format!("<|system|>\n</s>\n<|user|>\n{prompt}</s>\n<|assistant|>",)
                    } else {
                        format!("<|user|>\n{prompt}</s>\n<|assistant|>")
                    }
                } else if args.which.is_mistral() {
                    format!("[INST] {prompt} [/INST]")
                } else if args.which.is_deepseek() {
                    format!("<｜User｜>{prompt}<｜Assistant｜>")
                } else {
                    prompt
                }
            }
        };
        print!("{}", &prompt_str);
        let tokens = tos
            .tokenizer()
            .encode(prompt_str, true)
            .map_err(anyhow::Error::msg)?;
        if args.verbose_prompt {
            for (token, id) in tokens.get_tokens().iter().zip(tokens.get_ids().iter()) {
                let token = token.replace('▁', " ").replace("<0x0A>", "\n");
                println!("{id:7} -> '{token}'");
            }
        }

        let prompt_tokens = [&pre_prompt_tokens, tokens.get_ids()].concat();
        let to_sample = args.sample_len.saturating_sub(1);
        let prompt_tokens = if prompt_tokens.len() + to_sample > model::MAX_SEQ_LEN - 10 {
            let to_remove = prompt_tokens.len() + to_sample + 10 - model::MAX_SEQ_LEN;
            prompt_tokens[prompt_tokens.len().saturating_sub(to_remove)..].to_vec()
        } else {
            prompt_tokens
        };
        let mut all_tokens = vec![];
        let mut logits_processor = {
            let temperature = args.temperature;
            let sampling = if temperature <= 0. {
                Sampling::ArgMax
            } else {
                match (args.top_k, args.top_p) {
                    (None, None) => Sampling::All { temperature },
                    (Some(k), None) => Sampling::TopK { k, temperature },
                    (None, Some(p)) => Sampling::TopP { p, temperature },
                    (Some(k), Some(p)) => Sampling::TopKThenTopP { k, p, temperature },
                }
            };
            LogitsProcessor::from_sampling(args.seed, sampling)
        };

        let start_prompt_processing = std::time::Instant::now();
        let mut next_token = if !args.split_prompt {
            let input = Tensor::new(prompt_tokens.as_slice(), &device)?.unsqueeze(0)?;
            let logits = model.forward(&input, 0)?;
            let logits = logits.squeeze(0)?;
            logits_processor.sample(&logits)?
        } else {
            let mut next_token = 0;
            for (pos, token) in prompt_tokens.iter().enumerate() {
                let input = Tensor::new(&[*token], &device)?.unsqueeze(0)?;
                let logits = model.forward(&input, pos)?;
                let logits = logits.squeeze(0)?;
                next_token = logits_processor.sample(&logits)?
            }
            next_token
        };
        let prompt_dt = start_prompt_processing.elapsed();
        all_tokens.push(next_token);
        if let Some(t) = tos.next_token(next_token)? {
            print!("{t}");
            std::io::stdout().flush()?;
        }

        let eos_token = match args.which {
            Which::SmolLM2_360MInstruct | Which::SmolLM2_1BInstruct => "<|endoftext|>",
            Which::L8b => "<|end_of_text|>",
            Which::DeepseekR1Llama8b => "<｜end▁of▁sentence｜>",
            _ => match args.which.is_open_chat() {
                true => "<|end_of_turn|>",
                false => "</s>",
            },
        };

        let eos_token = *tos.tokenizer().get_vocab(true).get(eos_token).unwrap();
        let start_post_prompt = std::time::Instant::now();
        let mut sampled = 0;
        for index in 0..to_sample {
            let input = Tensor::new(&[next_token], &device)?.unsqueeze(0)?;
            let logits = model.forward(&input, prompt_tokens.len() + index)?;
            let logits = logits.squeeze(0)?;
            let logits = if args.repeat_penalty == 1. {
                logits
            } else {
                let start_at = all_tokens.len().saturating_sub(args.repeat_last_n);
                candle_transformers::utils::apply_repeat_penalty(
                    &logits,
                    args.repeat_penalty,
                    &all_tokens[start_at..],
                )?
            };
            next_token = logits_processor.sample(&logits)?;
            all_tokens.push(next_token);
            if let Some(t) = tos.next_token(next_token)? {
                print!("{t}");
                std::io::stdout().flush()?;
            }
            sampled += 1;
            if next_token == eos_token {
                break;
            };
        }
        if let Some(rest) = tos.decode_rest().map_err(candle::Error::msg)? {
            print!("{rest}");
        }
        std::io::stdout().flush()?;
        let dt = start_post_prompt.elapsed();
        println!(
            "\n\n{:4} prompt tokens processed: {:.2} token/s",
            prompt_tokens.len(),
            prompt_tokens.len() as f64 / prompt_dt.as_secs_f64(),
        );
        println!(
            "{sampled:4} tokens generated: {:.2} token/s",
            sampled as f64 / dt.as_secs_f64(),
        );

        match prompt {
            Prompt::One(_) => break,
            Prompt::Interactive => {}
            Prompt::Chat => {
                pre_prompt_tokens = [prompt_tokens.as_slice(), all_tokens.as_slice()].concat()
            }
        }
    }

    Ok(())
}
