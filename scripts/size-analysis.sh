#!/bin/bash
# Licensed under the Apache-2.0 license
# Binary size analysis script for local development
# Usage: ./scripts/size-analysis.sh [baseline-branch]

set -e

BASELINE_BRANCH=${1:-main}
CURRENT_BRANCH=$(git rev-parse --abbrev-ref HEAD)

echo "🔍 Analyzing binary size changes..."
echo "Baseline: $BASELINE_BRANCH"
echo "Current:  $CURRENT_BRANCH"
echo ""

# Function to build and analyze
analyze_size() {
    local branch=$1
    local output_file=$2
    
    echo "Building $branch..."
    git checkout $branch >/dev/null 2>&1
    cargo build --release --target thumbv7em-none-eabihf >/dev/null 2>&1
    
    # Check if cargo bloat is available
    if command -v cargo-bloat >/dev/null 2>&1; then
        cargo bloat --release --target thumbv7em-none-eabihf -n 20 > $output_file 2>/dev/null || {
            echo "cargo bloat failed, using basic size info"
            ls -la target/thumbv7em-none-eabihf/release/aspeed-ddk > $output_file
        }
    else
        echo "cargo-bloat not found, install with: cargo install cargo-bloat"
        ls -la target/thumbv7em-none-eabihf/release/aspeed-ddk > $output_file
    fi
}

# Store current branch
ORIGINAL_BRANCH=$CURRENT_BRANCH

# Analyze baseline
analyze_size $BASELINE_BRANCH target/baseline-size.txt

# Analyze current branch
analyze_size $CURRENT_BRANCH target/current-size.txt

# Restore original branch
git checkout $ORIGINAL_BRANCH >/dev/null 2>&1

echo "📊 Size Analysis Results:"
echo "========================"

# Extract binary sizes if using cargo bloat
if grep -q "File" target/baseline-size.txt 2>/dev/null; then
    BASELINE_SIZE=$(grep "File" target/baseline-size.txt | awk '{print $3}' | head -1)
    CURRENT_SIZE=$(grep "File" target/current-size.txt | awk '{print $3}' | head -1)
    
    if [[ $BASELINE_SIZE =~ ([0-9.]+)([A-Za-z]+) ]]; then
        echo "Baseline size: $BASELINE_SIZE"
    fi
    if [[ $CURRENT_SIZE =~ ([0-9.]+)([A-Za-z]+) ]]; then
        echo "Current size:  $CURRENT_SIZE"
    fi
else
    # Fallback to file size
    BASELINE_BYTES=$(stat -c%s target/thumbv7em-none-eabihf/release/aspeed-ddk 2>/dev/null || echo "0")
    echo "Binary size: $BASELINE_BYTES bytes"
fi

echo ""
echo "Top functions in current build:"
echo "==============================="
head -20 target/current-size.txt

echo ""
echo "Full reports saved in:"
echo "- target/baseline-size.txt"
echo "- target/current-size.txt"
