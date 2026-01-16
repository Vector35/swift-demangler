# swift-demangler

Idiomatic Rust bindings for Swift symbol demangling.

This crate wraps a vendored copy of the Swift runtime's demangling code, providing safe, ergonomic Rust APIs for:

- Demangling Swift symbols to human-readable strings
- Traversing the full parse tree with proper lifetime management
- Extracting function metadata (async, throws, parameters, return type, etc.)

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
swift-demangler = "0.1"
```

### Simple Demangling

```rust
use swift_demangler::demangle;

let demangled = demangle("$s4main5helloSSyYaKF").unwrap();
assert_eq!(demangled, "main.hello() async throws -> Swift.String");
```

### Structured Symbol Analysis

```rust
use swift_demangler::{Context, HasFunctionSignature, HasModule, Symbol};

let ctx = Context::new();
if let Some(symbol) = Symbol::parse(&ctx, "$s4main5helloSSyYaKF") {
    if let Some(func) = symbol.as_function() {
        println!("Function: {}", func.name().unwrap_or("?"));
        println!("Module: {}", func.module().unwrap_or("?"));
        println!("Async: {}", func.is_async());
        println!("Throws: {}", func.is_throwing());
    }
}
```

### Node Tree Traversal

For detailed analysis, the `raw` module provides direct access to the parse tree:

```rust
use swift_demangler::Context;
use swift_demangler::raw::{Node, NodeKind};

let ctx = Context::new();
if let Some(root) = Node::parse(&ctx, "$s4main5helloSSyYaKF") {
    for node in root.descendants() {
        println!("{:?}: {:?}", node.kind(), node.text());
    }
}
```

## Building

### Default (Bundled)

By default, the crate builds the C++ library from source using CMake:

```bash
cargo build
```

This requires:
- CMake 3.16+
- C++20 compiler (Clang or GCC)

### Pre-built Library

To link against a pre-built library instead of building from source:

```bash
# Build the C library separately
cmake -S swift-demangling -B build -G Ninja
cmake --build build

# Link the Rust crate to the pre-built library
SWIFT_DEMANGLE_DIR=build/lib cargo build --no-default-features
```

### Feature Flags

- `bundled` (default) - Build the C++ library from source using CMake
- `cli` - Build the `swift-demangler` command-line tool

### CLI Tool

```bash
cargo build --features cli --release
./target/release/swift-demangler '$s4main5helloSSyYaKF'
./target/release/swift-demangler --tree '$s4main5helloSSyYaKF'
./target/release/swift-demangler --function '$s4main5helloSSyYaKF'
```

## License

This crate is licensed under the Apache License 2.0.

It includes vendored code from the Swift and LLVM projects:

- **Swift**: Apache License 2.0 - see `swift-demangling/vendor/LICENSE-Swift.txt`
- **LLVM**: Apache License 2.0 with LLVM Exceptions - see `swift-demangling/vendor/LICENSE-LLVM.txt`
