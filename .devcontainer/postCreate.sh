#!/bin/bash
# Post-create script for devcontainer
# Fixes cargo permissions and sets up environment

set -e

echo "Setting up cargo environment..."

# Use user-local cargo directory to avoid permission issues
export CARGO_HOME=$HOME/.cargo
mkdir -p $CARGO_HOME

# Add to bashrc/zshrc for persistence
if [ -f "$HOME/.bashrc" ]; then
    if ! grep -q "CARGO_HOME" "$HOME/.bashrc"; then
        echo "export CARGO_HOME=\$HOME/.cargo" >> "$HOME/.bashrc"
    fi
fi

if [ -f "$HOME/.zshrc" ]; then
    if ! grep -q "CARGO_HOME" "$HOME/.zshrc"; then
        echo "export CARGO_HOME=\$HOME/.cargo" >> "$HOME/.zshrc"
    fi
fi

echo "✅ Cargo environment configured"

