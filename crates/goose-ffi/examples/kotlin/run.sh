#!/bin/bash

# Build the Rust library first
echo "Building Goose FFI library..."
cd ../..
cargo build
cd examples/kotlin

# Download JNA if not present
if [ ! -f "jna.jar" ]; then
    echo "Downloading JNA..."
    curl -L -o jna.jar https://repo1.maven.org/maven2/net/java/dev/jna/jna/5.13.0/jna-5.13.0.jar
fi

# Compile and run the Kotlin example
echo "Compiling Kotlin example..."
kotlinc -cp jna.jar GooseExample.kt -include-runtime -d goose-example.jar

echo "Running Kotlin example..."
java -cp "goose-example.jar:jna.jar" GooseExampleKt
