#!/bin/bash
# Run test coverage analysis

set -e

echo "Running test coverage analysis..."

# Install cargo-llvm-cov if not present
if ! command -v cargo-llvm-cov &> /dev/null; then
    echo "Installing cargo-llvm-cov..."
    cargo install cargo-llvm-cov
fi

# Run tests with coverage
echo "Running tests with coverage..."
cargo llvm-cov --all-features --tests --lcov --output-path lcov.info

# Generate HTML report
echo "Generating HTML report..."
cargo llvm-cov --all-features --tests --html

echo "Coverage report generated in target/llvm-cov/html/index.html"
echo "LCOV report: lcov.info"

