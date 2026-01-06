#!/bin/bash
# Verify tests can compile and identify issues

set -e

echo "=== Verifying Test Compilation ==="

# Check if we can compile tests
echo "Checking test compilation..."
if cargo test --no-run --lib completeness 2>&1 | tee /tmp/test_compile.log; then
    echo "✅ Library tests compile successfully"
else
    echo "❌ Library tests failed to compile"
    echo "Errors:"
    grep -i "error" /tmp/test_compile.log | head -20
    exit 1
fi

# Check integration tests
echo "Checking integration tests..."
if cargo test --no-run --test '*' 2>&1 | tee /tmp/integration_compile.log; then
    echo "✅ Integration tests compile successfully"
else
    echo "❌ Integration tests failed to compile"
    echo "Errors:"
    grep -i "error" /tmp/integration_compile.log | head -20
    exit 1
fi

echo ""
echo "=== Running Tests ==="
echo "Running library tests..."
cargo test --lib completeness

echo "Running integration tests..."
cargo test --test '*'

echo ""
echo "✅ All tests passed!"

