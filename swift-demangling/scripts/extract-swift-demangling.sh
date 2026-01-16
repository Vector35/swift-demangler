#!/bin/bash
# extract-swift-demangling.sh
# Run from the project root with Swift checkout path as argument

set -e

if [ -z "$1" ]; then
    echo "Usage: $0 /path/to/swift"
    exit 1
fi

SWIFT_SRC="$1"
LLVM_SRC="$(realpath "$SWIFT_SRC/../llvm-project/llvm" 2>/dev/null)"

if [ ! -d "$SWIFT_SRC" ]; then
    echo "Error: Swift source directory not found: $SWIFT_SRC"
    exit 1
fi

if [ -z "$LLVM_SRC" ] || [ ! -d "$LLVM_SRC/include/llvm/ADT" ]; then
    echo "Error: LLVM source not found at ../llvm-project/llvm relative to Swift"
    exit 1
fi

echo "Extracting Swift demangling from: $SWIFT_SRC"
echo "Using LLVM from: $LLVM_SRC"

DEST="vendor/swift-demangling"
rm -rf "$DEST"
mkdir -p "$DEST/lib" "$DEST/include/swift"

# Core source files
echo "Copying core demangling sources..."
cp "$SWIFT_SRC/lib/Demangling/"*.cpp "$DEST/lib/"
cp "$SWIFT_SRC/lib/Demangling/"*.h "$DEST/lib/" 2>/dev/null || true

# Public headers
echo "Copying public headers..."
cp -r "$SWIFT_SRC/include/swift/Demangling" "$DEST/include/swift/"
cp "$SWIFT_SRC/include/swift/Strings.h" "$DEST/include/swift/"

# Required Basic headers
echo "Copying Basic headers..."
mkdir -p "$DEST/include/swift/Basic"
for f in LLVM.h STLExtras.h Compiler.h FlagSet.h EnumeratedArray.h \
         InlineBitfield.h OptionalEnum.h OptionSet.h TaggedUnion.h \
         Unreachable.h MacroRoles.def; do
  cp "$SWIFT_SRC/include/swift/Basic/$f" "$DEST/include/swift/Basic/" 2>/dev/null || true
done

# Required AST headers (for NodePrinter)
echo "Copying AST headers..."
mkdir -p "$DEST/include/swift/AST"
for f in Ownership.h ReferenceStorage.def AttrKind.h LayoutConstraint.h \
         LayoutConstraintKind.h RequirementKind.h; do
  cp "$SWIFT_SRC/include/swift/AST/$f" "$DEST/include/swift/AST/" 2>/dev/null || true
done

# Required ABI headers (including .def files)
echo "Copying ABI headers..."
mkdir -p "$DEST/include/swift/ABI"
cp "$SWIFT_SRC/include/swift/ABI/"*.h "$DEST/include/swift/ABI/" 2>/dev/null || true
cp "$SWIFT_SRC/include/swift/ABI/"*.def "$DEST/include/swift/ABI/" 2>/dev/null || true

# Runtime stubs (minimal)
echo "Creating runtime stubs..."
mkdir -p "$DEST/include/swift/Runtime"
cat > "$DEST/include/swift/Runtime/Atomic.h" << 'EOF'
#ifndef SWIFT_RUNTIME_ATOMIC_H
#define SWIFT_RUNTIME_ATOMIC_H
#include <atomic>
namespace swift {
template<typename T> using Atomic = std::atomic<T>;
}
#endif
EOF

cat > "$DEST/include/swift/Runtime/Portability.h" << 'EOF'
#ifndef SWIFT_RUNTIME_PORTABILITY_H
#define SWIFT_RUNTIME_PORTABILITY_H
// Minimal stub
#endif
EOF

# Shims
echo "Creating shims..."
mkdir -p "$DEST/include/swift/shims"
cat > "$DEST/include/swift/shims/Visibility.h" << 'EOF'
#ifndef SWIFT_SHIMS_VISIBILITY_H
#define SWIFT_SHIMS_VISIBILITY_H
#define SWIFT_RUNTIME_EXPORT
#define SWIFT_CC(CC)
#endif
EOF

# ============================================================================
# LLVM Headers (copy real headers, stub only the generated config)
# ============================================================================
echo "Copying LLVM headers..."

LLVM_DEST="$DEST/include/llvm"
mkdir -p "$LLVM_DEST/ADT" "$LLVM_DEST/Support" "$LLVM_DEST/Config"

# Direct LLVM dependencies from Swift demangling code:
#   ADT: ArrayRef, DenseMap, FoldingSet, Hashing, PointerIntPair,
#        SmallVector, STLExtras, StringRef, StringSwitch
#   Support: Alignment, Casting, Compiler, DataTypes, ErrorHandling,
#            MathExtras, type_traits
#   BinaryFormat: Swift.def

# Copy direct ADT dependencies + their transitive deps
echo "Copying LLVM ADT headers..."
ADT_HEADERS="
    StringRef.h ArrayRef.h SmallVector.h STLExtras.h Hashing.h
    DenseMap.h DenseMapInfo.h DenseMapInfoVariant.h FoldingSet.h
    PointerIntPair.h StringSwitch.h
    iterator_range.h iterator.h STLForwardCompat.h STLFunctionalExtras.h
    ADL.h EpochTracker.h SmallString.h Twine.h
    PointerUnion.h None.h Optional.h bit.h identity.h
    fallible_iterator.h PointerSumType.h
"
for f in $ADT_HEADERS; do
    [ -f "$LLVM_SRC/include/llvm/ADT/$f" ] && cp "$LLVM_SRC/include/llvm/ADT/$f" "$LLVM_DEST/ADT/"
done

# Copy direct Support dependencies + their transitive deps
echo "Copying LLVM Support headers..."
SUPPORT_HEADERS="
    Compiler.h DataTypes.h ErrorHandling.h MathExtras.h type_traits.h
    Casting.h Alignment.h PointerLikeTypeTraits.h TypeSize.h
    SwapByteOrder.h raw_ostream.h Format.h NativeFormatting.h
    AlignOf.h MemAlloc.h ReverseIteration.h
"
for f in $SUPPORT_HEADERS; do
    [ -f "$LLVM_SRC/include/llvm/Support/$f" ] && cp "$LLVM_SRC/include/llvm/Support/$f" "$LLVM_DEST/Support/"
done

# Copy llvm-c headers (DataTypes.h dependency)
echo "Copying llvm-c headers..."
mkdir -p "$DEST/include/llvm-c"
cp "$LLVM_SRC/include/llvm-c/DataTypes.h" "$DEST/include/llvm-c/"

# Copy BinaryFormat (Swift.def)
echo "Copying LLVM BinaryFormat..."
mkdir -p "$LLVM_DEST/BinaryFormat"
cp "$LLVM_SRC/include/llvm/BinaryFormat/Swift.def" "$LLVM_DEST/BinaryFormat/" 2>/dev/null || true

# Stub only the generated config files
cat > "$LLVM_DEST/Config/llvm-config.h" << 'EOF'
#ifndef LLVM_CONFIG_H
#define LLVM_CONFIG_H
// Stub for generated LLVM config - only defines what Swift demangling needs
#if defined(__APPLE__)
#define LLVM_ON_UNIX 1
#elif defined(__linux__)
#define LLVM_ON_UNIX 1
#elif defined(_WIN32)
#define LLVM_ON_WIN32 1
#else
#define LLVM_ON_UNIX 1
#endif
#define LLVM_VERSION_MAJOR 18
#define LLVM_VERSION_MINOR 0
#define LLVM_VERSION_PATCH 0
#define LLVM_VERSION_STRING "18.0.0"
#define LLVM_ENABLE_ABI_BREAKING_CHECKS 0
#endif
EOF

cat > "$LLVM_DEST/Config/abi-breaking.h" << 'EOF'
#ifndef LLVM_CONFIG_ABI_BREAKING_H
#define LLVM_CONFIG_ABI_BREAKING_H
#define LLVM_ENABLE_ABI_BREAKING_CHECKS 0
#define LLVM_ENABLE_REVERSE_ITERATION 0
#endif
EOF

# Copy license files alongside their source files
echo "Copying license files..."
cp "$SWIFT_SRC/LICENSE.txt" "$DEST/LICENSE-Swift.txt"
cp "$LLVM_SRC/../LICENSE.TXT" "$DEST/LICENSE-LLVM.txt"

# Copy test data
echo "Copying test data..."
mkdir -p "$DEST/tests"
cp "$SWIFT_SRC/test/Demangle/Inputs/manglings.txt" "$DEST/tests/"
cp "$SWIFT_SRC/test/Demangle/Inputs/simplified-manglings.txt" "$DEST/tests/"

# Record version
cd "$SWIFT_SRC"
VERSION_INFO=$(git describe --tags 2>/dev/null || git rev-parse --short HEAD)
echo "$VERSION_INFO" > "$OLDPWD/$DEST/VERSION"
cd "$OLDPWD"

# Generate node kind enums (C and Rust)
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
"$SCRIPT_DIR/generate-node-kinds.sh" "$DEST"

echo ""
echo "============================================"
echo "Extracted Swift demangling from: $VERSION_INFO"
echo "Output directory: $DEST"
echo "============================================"
echo ""
echo "Files extracted:"
find "$DEST" -type f | head -30
TOTAL=$(find "$DEST" -type f | wc -l)
echo "... ($TOTAL files total)"
