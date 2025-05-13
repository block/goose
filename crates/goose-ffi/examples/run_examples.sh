#!/bin/bash
# Run all Goose FFI examples

echo "Goose FFI Examples Runner"
echo "========================"
echo

# Check if environment variables are set
if [ -z "$DATABRICKS_API_KEY" ] || [ -z "$DATABRICKS_HOST" ]; then
    echo "Error: Please set DATABRICKS_API_KEY and DATABRICKS_HOST environment variables"
    exit 1
fi

# Build the project if needed
echo "Building Goose FFI library..."
cd ../../.. && cargo build
cd crates/goose-ffi/examples

# Run tests
echo
echo "Running FFI tests..."
echo "-------------------"
python3 test_ffi.py

# Run examples
echo
echo "Running basic example..."
echo "----------------------"
echo "Type 'quit' to exit the example"
python3 goose_agent.py

echo
echo "Running end-to-end example..."
echo "---------------------------"
python3 end_to_end_example.py

echo
echo "All examples completed!"