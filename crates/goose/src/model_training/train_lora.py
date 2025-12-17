#!/usr/bin/env python3
"""
Lightweight LoRA training script using transformers + peft.
This replaces Axolotl for Python 3.14 compatibility.
"""

import argparse
import json
import logging
import os
import sys
from pathlib import Path
from typing import Dict, List

import torch
from transformers import (
    AutoModelForCausalLM,
    AutoTokenizer,
    TrainingArguments,
    Trainer,
    DataCollatorForLanguageModeling,
)
from peft import LoraConfig, get_peft_model, TaskType
from datasets import Dataset

logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger(__name__)


def load_config(config_path: str) -> Dict:
    """Load training configuration from YAML or JSON."""
    import yaml
    
    with open(config_path, 'r') as f:
        if config_path.endswith('.json'):
            return json.load(f)
        else:
            return yaml.safe_load(f)


def load_dataset_from_jsonl(dataset_path: str) -> List[Dict]:
    """Load chat dataset from JSONL file."""
    data = []
    with open(dataset_path, 'r') as f:
        for line in f:
            if line.strip():
                data.append(json.loads(line))
    return data


def format_chat_messages(messages: List[Dict], tokenizer) -> str:
    """Format chat messages using tokenizer's chat template."""
    if hasattr(tokenizer, 'apply_chat_template'):
        return tokenizer.apply_chat_template(
            messages,
            tokenize=False,
            add_generation_prompt=False
        )
    else:
        # Fallback: simple concatenation
        formatted = ""
        for msg in messages:
            role = msg.get('role', 'user')
            content = msg.get('content', '')
            formatted += f"<|{role}|>\n{content}\n"
        return formatted


def prepare_dataset(data: List[Dict], tokenizer, max_length: int = 2048):
    """Prepare dataset for training."""
    formatted_texts = []
    
    for item in data:
        messages = item.get('messages', [])
        if messages:
            text = format_chat_messages(messages, tokenizer)
            formatted_texts.append(text)
    
    # Tokenize
    def tokenize_function(examples):
        return tokenizer(
            examples['text'],
            truncation=True,
            max_length=max_length,
            padding='max_length',
            return_tensors=None
        )
    
    # Create dataset
    dataset = Dataset.from_dict({'text': formatted_texts})
    tokenized_dataset = dataset.map(
        tokenize_function,
        batched=True,
        remove_columns=['text']
    )
    
    return tokenized_dataset


def main():
    parser = argparse.ArgumentParser(description='Train LoRA adapter')
    parser.add_argument('-c', '--config', required=True, help='Path to config file')
    args = parser.parse_args()
    
    # Load configuration
    logger.info(f"Loading config from {args.config}")
    config = load_config(args.config)
    
    base_model = config['base_model']
    output_dir = config['output_dir']
    datasets_config = config.get('datasets', [])
    lora_config = config.get('lora', {})
    training_config = config.get('training', {})
    hf_token = config.get('hf_token')  # HuggingFace token for gated models
    use_cpu = config.get('use_cpu', False)  # Force CPU training
    
    logger.info(f"Base model: {base_model}")
    logger.info(f"Output directory: {output_dir}")
    
    # Set HuggingFace token if provided
    if hf_token:
        logger.info("Using HuggingFace authentication token")
        os.environ['HF_TOKEN'] = hf_token
    
    # Create output directory
    Path(output_dir).mkdir(parents=True, exist_ok=True)
    
    # Determine device and memory settings
    if use_cpu:
        logger.info("Forcing CPU training (memory-constrained mode)")
        device_map = None
        torch_dtype = torch.float32
        use_mps = False
    elif torch.cuda.is_available():
        logger.info("Using CUDA GPU")
        device_map = 'auto'
        torch_dtype = torch.float16
        use_mps = False
    elif torch.backends.mps.is_available():
        logger.info("Using Apple MPS GPU")
        device_map = None  # MPS doesn't support device_map='auto'
        torch_dtype = torch.float32  # MPS has issues with float16
        use_mps = True
    else:
        logger.info("Using CPU")
        device_map = None
        torch_dtype = torch.float32
        use_mps = False
    
    # Load tokenizer and model
    logger.info("Loading tokenizer and model...")
    tokenizer_kwargs = {'trust_remote_code': True}
    if hf_token:
        tokenizer_kwargs['token'] = hf_token
    
    tokenizer = AutoTokenizer.from_pretrained(base_model, **tokenizer_kwargs)
    
    # Set pad token if not set
    if tokenizer.pad_token is None:
        tokenizer.pad_token = tokenizer.eos_token
    
    model_kwargs = {
        'torch_dtype': torch_dtype,
        'device_map': device_map,
        'trust_remote_code': True,
    }
    if hf_token:
        model_kwargs['token'] = hf_token
    
    # For MPS, load to CPU first then move to MPS
    if use_mps:
        model_kwargs['device_map'] = None
        model = AutoModelForCausalLM.from_pretrained(base_model, **model_kwargs)
        model = model.to('mps')
    else:
        model = AutoModelForCausalLM.from_pretrained(base_model, **model_kwargs)
    
    # Configure LoRA
    logger.info("Configuring LoRA...")
    peft_config = LoraConfig(
        task_type=TaskType.CAUSAL_LM,
        r=lora_config.get('r', 16),
        lora_alpha=lora_config.get('alpha', 32),
        lora_dropout=lora_config.get('dropout', 0.05),
        target_modules=lora_config.get('target_modules', ['q_proj', 'k_proj', 'v_proj', 'o_proj']),
        bias=lora_config.get('bias', 'none'),
    )
    
    model = get_peft_model(model, peft_config)
    model.print_trainable_parameters()
    
    # Load and prepare datasets
    logger.info("Loading datasets...")
    all_data = []
    for ds_config in datasets_config:
        ds_path = ds_config['path']
        ds_type = ds_config.get('type', 'chat')
        
        if ds_type == 'chat':
            data = load_dataset_from_jsonl(ds_path)
            all_data.extend(data)
            logger.info(f"Loaded {len(data)} examples from {ds_path}")
    
    if not all_data:
        logger.error("No training data loaded!")
        sys.exit(1)
    
    logger.info(f"Total training examples: {len(all_data)}")
    
    # Prepare dataset
    train_dataset = prepare_dataset(
        all_data,
        tokenizer,
        max_length=training_config.get('max_seq_length', 2048)
    )
    
    # Training arguments
    logger.info("Setting up training...")
    
    # Adjust batch size for memory constraints
    batch_size = training_config.get('per_device_train_batch_size', 2)
    if use_cpu or use_mps:
        # Use smaller batch size for CPU/MPS to avoid OOM
        batch_size = min(batch_size, 1)
        logger.info(f"Using reduced batch size {batch_size} for memory-constrained device")
    
    training_args = TrainingArguments(
        output_dir=output_dir,
        num_train_epochs=training_config.get('epochs', 3),
        per_device_train_batch_size=batch_size,
        gradient_accumulation_steps=training_config.get('gradient_accumulation_steps', 4 if use_mps else 1),
        learning_rate=training_config.get('learning_rate', 2e-4),
        warmup_steps=training_config.get('warmup_steps', 10),
        logging_steps=training_config.get('logging_steps', 10),
        save_steps=training_config.get('save_steps', 50),
        save_total_limit=3,
        fp16=training_config.get('fp16', False) and torch.cuda.is_available(),
        bf16=training_config.get('bf16', False) and torch.cuda.is_available(),
        optim='adamw_torch',
        weight_decay=training_config.get('weight_decay', 0.0),
        logging_first_step=True,
        report_to='none',  # Disable wandb, tensorboard, etc.
        remove_unused_columns=False,
        use_cpu=use_cpu,  # Force CPU if requested
    )
    
    # Data collator
    data_collator = DataCollatorForLanguageModeling(
        tokenizer=tokenizer,
        mlm=False,
    )
    
    # Trainer
    trainer = Trainer(
        model=model,
        args=training_args,
        train_dataset=train_dataset,
        data_collator=data_collator,
    )
    
    # Train
    logger.info("Starting training...")
    trainer.train()
    
    # Save model
    logger.info(f"Saving model to {output_dir}")
    trainer.save_model(output_dir)
    tokenizer.save_pretrained(output_dir)
    
    logger.info("Training complete!")


if __name__ == '__main__':
    main()
