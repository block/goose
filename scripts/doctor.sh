#!/bin/bash

echo "Running Goose development environment doctor..."
echo "------------------------------------------------"
all_ok=true

# Check 1: Does the hermit binary exist and is it executable?
if [ -x "bin/hermit" ]; then
    echo "'bin/hermit' executable found."
else
    echo "'bin/hermit' not found or not executable. Please ensure you are in the root of the repository."
    all_ok=false
fi

# Check 2: Is the Hermit environment active?
# Hermit sets the HERMIT_ENV variable when activated. This is the most reliable check.
if [ -n "$HERMIT_ENV" ]; then
    echo "Hermit environment is active."
else
    echo "Hermit environment is NOT active."
    echo "   To activate it, run the following in your shell and then try again:"
    echo '   eval "$(bin/hermit env)"'
    all_ok=false
fi

echo "------------------------------------------------"

if [ "$all_ok" = true ]; then
    echo "Your environment is ready! All tools (rustc, uv, just) are now available through Hermit."
    exit 0
else
    echo "Your environment is not fully configured. Please follow the instructions above."
    exit 1
fi
