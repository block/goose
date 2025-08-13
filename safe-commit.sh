#!/bin/bash
# Safe commit helper script
# Usage: ./safe-commit.sh "your commit message"

echo "🔍 Checking what will be committed..."
git status --porcelain

echo ""
echo "📋 Files that would be staged:"
git diff --name-only

echo ""
echo "⚠️  Checking for .gitignore changes..."
if git diff --name-only | grep -q "\.gitignore"; then
    echo "❌ WARNING: .gitignore has changes that should NOT be committed!"
    echo "   Run: git restore .gitignore"
    echo "   Or manually exclude it from your commit"
    exit 1
fi

echo "✅ Safe to commit!"
if [ "$1" ]; then
    # Add all files except .gitignore
    git add .
    git restore --staged .gitignore 2>/dev/null || true
    echo "📝 Committing (excluding .gitignore)..."
    git commit --signoff -m "$1"
    echo "✅ Committed: $1"
else
    echo "Usage: $0 'your commit message'"
fi
