# Swift Demangle C Library

A standalone C library for demangling Swift symbols and extracting function metadata.

This library wraps Swift's internal demangling code and exposes a clean C API that hides all Swift/LLVM implementation details from consumers.

## Building

### Prerequisites

- CMake 3.16+
- Ninja (or another CMake generator)
- C++20 compiler (Clang or GCC)

### Quick Start

```bash
cmake -S swift-demangling -B build -G Ninja
cmake --build build

# Test
./build/swift-demangle '$s4main5helloSSyYaKF'
```

## API Usage

The library provides a C API through `swift_demangle.h`. Link against `libswift_demangle.a`.

### Simple Demangling

```c
#include "swift_demangle.h"

char* result = swift_demangle_symbol("$s4main5helloSSyYaKF");
// result = "main.hello() async throws -> Swift.String"
printf("%s\n", result);
swift_demangle_free_string(result);
```

### Node Tree Traversal

For detailed analysis, you can access the full parse tree:

```c
#include "swift_demangle.h"

struct SwiftDemangleContext* ctx = swift_demangle_context_create();
struct SwiftDemangleNode* root = swift_demangle_symbol_as_node(ctx, "$s4main5helloSSyYaKF");

if (root) {
    // Get the node kind
    SwiftNodeKind kind = swift_demangle_node_get_kind(root);
    const char* kind_name = swift_demangle_node_get_kind_name(root);

    // Check for text or index content
    if (swift_demangle_node_has_text(root)) {
        const char* text = swift_demangle_node_get_text(root);
    }

    // Traverse children
    size_t num_children = swift_demangle_node_get_num_children(root);
    for (size_t i = 0; i < num_children; i++) {
        struct SwiftDemangleNode* child = swift_demangle_node_get_child(root, i);
        // ... process child
    }
}

swift_demangle_context_destroy(ctx);
```

### Function Info Extraction

Extract structured information about function symbols:

```c
#include "swift_demangle.h"

SwiftFunctionInfo info;
if (swift_demangle_get_function_info("$s4main5helloSSyYaKF", &info)) {
    printf("Module: %s\n", info.module_name);      // "main"
    printf("Name: %s\n", info.function_name);      // "hello"
    printf("Async: %s\n", info.is_async ? "yes" : "no");
    printf("Throws: %s\n", info.is_throwing ? "yes" : "no");
    printf("Parameters: %zu\n", info.num_parameters);
    for (size_t i = 0; i < info.num_parameters; i++) {
        printf("  [%zu]: %s\n", i, info.parameter_types[i]);
    }
    printf("Return: %s\n", info.return_type);

    swift_function_info_destroy(&info);
}
```

### Node Kinds

All Swift demangling node kinds are available as the `SwiftNodeKind` enum in `swift_node_kinds.h`. Key node kinds for function analysis include:

- `SwiftNodeKind_Function`, `SwiftNodeKind_Constructor`, `SwiftNodeKind_Destructor`
- `SwiftNodeKind_AsyncAnnotation`, `SwiftNodeKind_ThrowsAnnotation`, `SwiftNodeKind_TypedThrowsAnnotation`
- `SwiftNodeKind_FunctionType`, `SwiftNodeKind_ArgumentTuple`, `SwiftNodeKind_ReturnType`
- `SwiftNodeKind_Module`, `SwiftNodeKind_Identifier`

### CLI Tool

The included `swift-demangle` CLI demonstrates the API:

```bash
# Simple demangling
./swift-demangle '$s4main5helloSSyYaKF'
# Output: Demangled: main.hello() async throws -> Swift.String

# Show full node tree
./swift-demangle --tree '$s4main5helloSSyYaKF'

# Show function info
./swift-demangle --function '$s4main5helloSSyYaKF'
```

## Updating Vendored Code

The library vendors Swift demangling code and required LLVM headers in `vendor/`. To update to a newer Swift version, you'll need:

- A Swift source checkout (e.g., from https://github.com/swiftlang/swift)
- LLVM source (typically at `../llvm-project` relative to Swift)

### 1. Extract from Swift Source

```bash
./scripts/extract-swift-demangling.sh /path/to/swift
```

The script will:
- Copy Swift demangling sources from `lib/Demangling/`
- Copy required Swift and LLVM headers
- Copy license files
- Generate `include/swift_node_kinds.h` from `DemangleNodes.def`
- Generate Rust bindings in `src/raw/node_kinds.rs`
- Record the Swift version in `vendor/VERSION`

The script expects LLVM source at `../llvm-project/llvm` relative to the Swift source directory (the standard layout for a Swift development checkout).

### 2. Rebuild

```bash
cmake --build build
```

The build includes static assertions that verify the generated `SwiftNodeKind` enum matches Swift's internal `Node::Kind` enum, so any mismatch will cause a compile error.

### Version Pinning

The extracted Swift version is recorded in `vendor/VERSION`. To ensure reproducible builds:

1. Check out a specific Swift release tag (e.g., `swift-6.0-RELEASE`)
2. Run the extraction script
3. Commit the vendored files

## License

This library wraps code from the Swift and LLVM projects.

- **Swift**: Apache License 2.0 - see `vendor/LICENSE-Swift.txt`
- **LLVM**: Apache License 2.0 with LLVM Exceptions - see `vendor/LICENSE-LLVM.txt`

The wrapper code in `src/` and `include/` may be used under the same Apache 2.0 license.
