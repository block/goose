#!/bin/bash

# Build Goose Swarm Whitepaper
# This script compiles the LaTeX whitepaper to PDF

set -e  # Exit on any error

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Get script directory
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
TEX_FILE="$SCRIPT_DIR/whitepaper.tex"
PDF_FILE="$SCRIPT_DIR/whitepaper.pdf"

echo -e "${YELLOW}Building Goose Swarm Whitepaper...${NC}"

# Check if pdflatex is installed
if ! command -v pdflatex &> /dev/null; then
    echo -e "${RED}Error: pdflatex is not installed.${NC}"
    echo "Please install a LaTeX distribution (e.g., MacTeX, TeX Live, MiKTeX)"
    exit 1
fi

# Check if the .tex file exists
if [ ! -f "$TEX_FILE" ]; then
    echo -e "${RED}Error: whitepaper.tex not found at $TEX_FILE${NC}"
    exit 1
fi

# Change to the directory containing the .tex file
cd "$SCRIPT_DIR"

# First pass
echo -e "${YELLOW}Running first LaTeX pass...${NC}"
pdflatex -interaction=nonstopmode -halt-on-error whitepaper.tex > /dev/null 2>&1 || {
    echo -e "${RED}Error during first LaTeX pass. Running with verbose output:${NC}"
    pdflatex whitepaper.tex
    exit 1
}

# Second pass (for references and TOC)
echo -e "${YELLOW}Running second LaTeX pass...${NC}"
pdflatex -interaction=nonstopmode -halt-on-error whitepaper.tex > /dev/null 2>&1 || {
    echo -e "${RED}Error during second LaTeX pass.${NC}"
    exit 1
}

# Clean up auxiliary files
echo -e "${YELLOW}Cleaning up auxiliary files...${NC}"
rm -f whitepaper.aux whitepaper.log whitepaper.out whitepaper.toc whitepaper.lof whitepaper.lot

# Check if PDF was created
if [ -f "$PDF_FILE" ]; then
    echo -e "${GREEN}✓ Whitepaper built successfully!${NC}"
    echo -e "${GREEN}  PDF location: $PDF_FILE${NC}"
    
    # Optionally open the PDF (on macOS)
    if [[ "$OSTYPE" == "darwin"* ]]; then
        echo -e "${YELLOW}Opening PDF...${NC}"
        open "$PDF_FILE"
    fi
else
    echo -e "${RED}Error: PDF file was not created.${NC}"
    exit 1
fi
