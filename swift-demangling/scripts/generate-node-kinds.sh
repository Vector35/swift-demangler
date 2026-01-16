#!/bin/bash
# generate-node-kinds.sh
# Generates node kind enums for C and Rust from DemangleNodes.def

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
VENDOR_DIR="${1:-$PROJECT_DIR/vendor/swift-demangling}"

if [ ! -f "$VENDOR_DIR/include/swift/Demangling/DemangleNodes.def" ]; then
    echo "Error: DemangleNodes.def not found at: $VENDOR_DIR/include/swift/Demangling/DemangleNodes.def"
    echo "Run extract-swift-demangling.sh first"
    exit 1
fi

TEMP_DIR=$(mktemp -d)
trap "rm -rf $TEMP_DIR" EXIT

# Generate C header
echo "Generating include/swift_node_kinds.h..."
cc -I"$VENDOR_DIR/include" -o "$TEMP_DIR/gen-c" "$SCRIPT_DIR/generate-node-kinds.c"
"$TEMP_DIR/gen-c" > "$PROJECT_DIR/include/swift_node_kinds.h"

# Generate Rust module
echo "Generating rust/src/raw/node_kinds.rs..."
cc -I"$VENDOR_DIR/include" -o "$TEMP_DIR/gen-rs" "$SCRIPT_DIR/generate-node-kinds-rs.c"
mkdir -p "$PROJECT_DIR/rust/src/raw"
"$TEMP_DIR/gen-rs" > "$PROJECT_DIR/rust/src/raw/node_kinds.rs"

COUNT=$(grep -c 'SwiftNodeKind_' "$PROJECT_DIR/include/swift_node_kinds.h")
echo "Generated $COUNT node kinds"
