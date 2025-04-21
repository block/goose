#!/bin/bash

# Create extension directory in goose repo
mkdir -p ~/goose/extensions/notion

# Copy files with correct structure
cp -r ~/goose-notion/goose_notion/* ~/goose/extensions/notion/
cp ~/goose-notion/setup.py ~/goose/extensions/notion/
cp ~/goose-notion/pyproject.toml ~/goose/extensions/notion/
cp ~/goose-notion/README.md ~/goose/extensions/notion/
cp ~/goose-notion/CONTRIBUTING.md ~/goose/extensions/notion/

echo "Files moved successfully!"