#!/bin/bash

# Create base directory for tokenizer files
BASE_DIR="tokenizer_files"
mkdir -p "$BASE_DIR"

# Function to download a tokenizer file
download_tokenizer() {
    local repo_id="$1"
    local dir_name="${repo_id//\/--}"  # Replace / with -- for directory name
    local download_dir="$BASE_DIR/${repo_id//\//--}"  # Replace / with -- for directory name, matching Python's replace("/", "--")
    local file_url="https://huggingface.co/$repo_id/resolve/main/tokenizer.json"
    
    mkdir -p "$download_dir"
    
    # Only download if the file doesn't exist
    if [ ! -f "$download_dir/tokenizer.json" ]; then
        echo "Downloading tokenizer for $repo_id..."
        curl -L "$file_url" -o "$download_dir/tokenizer.json"
        if [ $? -eq 0 ]; then
            echo "Downloaded $repo_id to $download_dir/tokenizer.json"
        else
            echo "Failed to download $repo_id tokenizer"
            return 1
        fi
    else
        echo "Tokenizer for $repo_id already exists, skipping..."
    fi
}

# Download tokenizers for each model
download_tokenizer "Xenova/gpt-4o"
download_tokenizer "Xenova/claude-tokenizer"
download_tokenizer "Qwen/Qwen2.5-Coder-32B-Instruct"