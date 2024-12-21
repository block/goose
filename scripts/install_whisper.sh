#!/bin/bash

# Create scripts directory if it doesn't exist
mkdir -p scripts

# Install Whisper.cpp
echo "Installing Whisper.cpp..."
git clone https://github.com/ggerganov/whisper.cpp.git
cd whisper.cpp

# Build the project
make

# Download the base English model
bash ./models/download-ggml-model.sh base.en

# Create a symbolic link to the whisper executable in /usr/local/bin
echo "Creating symbolic link to whisper executable..."
sudo ln -sf "$(pwd)/main" /usr/local/bin/whisper

echo "Whisper installation complete!"
echo "You can now use the 'whisper' command to transcribe audio files."
