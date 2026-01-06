#!/bin/bash
# Generate JSON schema files for IMACS output types

cd "$(dirname "$0")/.."

# Build the binary first
cargo build --release --bin imacs 2>/dev/null || cargo build --bin imacs 2>/dev/null

BIN="./target/release/imacs"
if [ ! -f "$BIN" ]; then
    BIN="./target/debug/imacs"
fi

if [ ! -f "$BIN" ]; then
    echo "Error: imacs binary not found. Please build first with: cargo build"
    exit 1
fi

mkdir -p schemas

echo "Generating schemas..."

$BIN schema spec > schemas/spec.schema.json
$BIN schema verify > schemas/verify.schema.json
$BIN schema analyze > schemas/analyze.schema.json
$BIN schema extract > schemas/extract.schema.json
$BIN schema drift > schemas/drift.schema.json
$BIN schema completeness > schemas/completeness.schema.json

echo "Schemas generated in schemas/ directory"

