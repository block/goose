#!/usr/bin/env python3
"""
LoRA Inference Server - OpenAI-compatible API for fine-tuned models.
Loads a base model with PEFT LoRA adapter and serves via HTTP.
"""

import argparse
import json
import logging
import os
import sys
import time
from pathlib import Path
from typing import Dict, List, Optional, Iterator

import torch
from transformers import (
    AutoModelForCausalLM,
    AutoTokenizer,
    TextIteratorStreamer,
)
from peft import PeftModel
from threading import Thread
from flask import Flask, request, jsonify, Response
from flask_cors import CORS

logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger(__name__)

app = Flask(__name__)
CORS(app)

# Global model and tokenizer
model = None
tokenizer = None
base_model_name = None
adapter_path = None


def load_model_with_adapter(base_model: str, adapter: str):
    """Load base model and apply LoRA adapter."""
    global model, tokenizer, base_model_name, adapter_path
    
    logger.info(f"Loading base model: {base_model}")
    base_model_name = base_model
    adapter_path = adapter
    
    # Load tokenizer
    tokenizer = AutoTokenizer.from_pretrained(base_model, trust_remote_code=True)
    if tokenizer.pad_token is None:
        tokenizer.pad_token = tokenizer.eos_token
    
    # Load base model
    device = "cuda" if torch.cuda.is_available() else "cpu"
    dtype = torch.float16 if torch.cuda.is_available() else torch.float32
    
    logger.info(f"Loading model on device: {device} with dtype: {dtype}")
    base = AutoModelForCausalLM.from_pretrained(
        base_model,
        torch_dtype=dtype,
        device_map='auto' if device == 'cuda' else None,
        trust_remote_code=True,
    )
    
    # Load and apply adapter
    logger.info(f"Loading LoRA adapter: {adapter}")
    model = PeftModel.from_pretrained(base, adapter)
    model.eval()
    
    logger.info("Model loaded successfully with adapter")
    return model, tokenizer


def format_chat_messages(messages: List[Dict]) -> str:
    """Format messages using tokenizer's chat template."""
    if hasattr(tokenizer, 'apply_chat_template'):
        return tokenizer.apply_chat_template(
            messages,
            tokenize=False,
            add_generation_prompt=True
        )
    else:
        # Fallback: Qwen2 format
        formatted = ""
        for msg in messages:
            role = msg.get('role', 'user')
            content = msg.get('content', '')
            formatted += f"<|im_start|>{role}\n{content}\n<|im_end|>\n"
        formatted += "<|im_start|>assistant\n"
        return formatted


def generate_response(
    prompt: str,
    max_tokens: int = 512,
    temperature: float = 0.7,
    top_p: float = 0.9,
    stream: bool = False
) -> Iterator[str]:
    """Generate response from model."""
    inputs = tokenizer(prompt, return_tensors="pt")
    
    if torch.cuda.is_available():
        inputs = {k: v.to('cuda') for k, v in inputs.items()}
    
    generation_kwargs = {
        "max_new_tokens": max_tokens,
        "temperature": temperature,
        "top_p": top_p,
        "do_sample": temperature > 0,
        "pad_token_id": tokenizer.pad_token_id,
        "eos_token_id": tokenizer.eos_token_id,
    }
    
    if stream:
        # Streaming generation
        streamer = TextIteratorStreamer(
            tokenizer,
            skip_prompt=True,
            skip_special_tokens=True
        )
        generation_kwargs["streamer"] = streamer
        
        # Generate in background thread
        thread = Thread(target=model.generate, kwargs={**inputs, **generation_kwargs})
        thread.start()
        
        # Yield tokens as they come
        for text in streamer:
            yield text
        
        thread.join()
    else:
        # Non-streaming generation
        with torch.no_grad():
            outputs = model.generate(**inputs, **generation_kwargs)
        
        # Decode only the generated part (skip input)
        generated = outputs[0][inputs['input_ids'].shape[1]:]
        text = tokenizer.decode(generated, skip_special_tokens=True)
        yield text


@app.route('/v1/chat/completions', methods=['POST'])
def chat_completions():
    """OpenAI-compatible chat completions endpoint."""
    try:
        data = request.json
        messages = data.get('messages', [])
        max_tokens = data.get('max_tokens', 512)
        temperature = data.get('temperature', 0.7)
        top_p = data.get('top_p', 0.9)
        stream = data.get('stream', False)
        
        if not messages:
            return jsonify({'error': 'No messages provided'}), 400
        
        # Format messages
        prompt = format_chat_messages(messages)
        
        if stream:
            # Streaming response
            def generate():
                response_id = f"chatcmpl-{int(time.time())}"
                
                for i, chunk in enumerate(generate_response(
                    prompt, max_tokens, temperature, top_p, stream=True
                )):
                    chunk_data = {
                        "id": response_id,
                        "object": "chat.completion.chunk",
                        "created": int(time.time()),
                        "model": f"{base_model_name}-lora",
                        "choices": [{
                            "index": 0,
                            "delta": {
                                "role": "assistant" if i == 0 else None,
                                "content": chunk
                            },
                            "finish_reason": None
                        }]
                    }
                    yield f"data: {json.dumps(chunk_data)}\n\n"
                
                # Final chunk
                final_chunk = {
                    "id": response_id,
                    "object": "chat.completion.chunk",
                    "created": int(time.time()),
                    "model": f"{base_model_name}-lora",
                    "choices": [{
                        "index": 0,
                        "delta": {},
                        "finish_reason": "stop"
                    }]
                }
                yield f"data: {json.dumps(final_chunk)}\n\n"
                yield "data: [DONE]\n\n"
            
            return Response(generate(), mimetype='text/event-stream')
        else:
            # Non-streaming response
            full_response = ''.join(generate_response(
                prompt, max_tokens, temperature, top_p, stream=False
            ))
            
            response = {
                "id": f"chatcmpl-{int(time.time())}",
                "object": "chat.completion",
                "created": int(time.time()),
                "model": f"{base_model_name}-lora",
                "choices": [{
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": full_response
                    },
                    "finish_reason": "stop"
                }],
                "usage": {
                    "prompt_tokens": len(tokenizer.encode(prompt)),
                    "completion_tokens": len(tokenizer.encode(full_response)),
                    "total_tokens": len(tokenizer.encode(prompt)) + len(tokenizer.encode(full_response))
                }
            }
            
            return jsonify(response)
    
    except Exception as e:
        logger.error(f"Error in chat_completions: {e}", exc_info=True)
        return jsonify({'error': str(e)}), 500


@app.route('/v1/models', methods=['GET'])
def list_models():
    """List available models."""
    return jsonify({
        "object": "list",
        "data": [{
            "id": f"{base_model_name}-lora",
            "object": "model",
            "created": int(time.time()),
            "owned_by": "distil",
            "permission": [],
            "root": base_model_name,
            "parent": None,
        }]
    })


@app.route('/health', methods=['GET'])
def health():
    """Health check endpoint."""
    return jsonify({
        "status": "ok",
        "model": base_model_name,
        "adapter": adapter_path,
        "device": "cuda" if torch.cuda.is_available() else "cpu"
    })


def main():
    parser = argparse.ArgumentParser(description='LoRA Inference Server')
    parser.add_argument('--base-model', required=True, help='Base model name or path')
    parser.add_argument('--adapter', required=True, help='Path to LoRA adapter')
    parser.add_argument('--port', type=int, default=8000, help='Port to run server on')
    parser.add_argument('--host', default='127.0.0.1', help='Host to bind to')
    args = parser.parse_args()
    
    # Validate adapter path
    adapter_path = Path(args.adapter)
    if not adapter_path.exists():
        logger.error(f"Adapter path does not exist: {adapter_path}")
        sys.exit(1)
    
    # Load model
    try:
        load_model_with_adapter(args.base_model, str(adapter_path))
    except Exception as e:
        logger.error(f"Failed to load model: {e}", exc_info=True)
        sys.exit(1)
    
    # Start server
    logger.info(f"Starting server on {args.host}:{args.port}")
    app.run(host=args.host, port=args.port, threaded=True)


if __name__ == '__main__':
    main()
