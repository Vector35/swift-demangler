/*
 * swift_demangle.h - Swift symbol demangling library
 *
 * A C API for demangling Swift symbols and extracting function metadata.
 * Hides all Swift/LLVM implementation details from consumers.
 */

#ifndef SWIFT_DEMANGLE_H
#define SWIFT_DEMANGLE_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include "swift_node_kinds.h"

#if defined(__GNUC__) || defined(__clang__)
#  define SWIFT_DEMANGLE_EXPORT __attribute__((visibility("default")))
#else
#  define SWIFT_DEMANGLE_EXPORT
#endif

#ifdef __cplusplus
extern "C" {
#endif

/* Opaque handles */
struct SwiftDemangleContext;
struct SwiftDemangleNode;

/* A non-owning view of a string (ptr + length) */
typedef struct {
    const char* data;  /* Pointer to the string data (not null-terminated) */
    size_t length;     /* Length of the string in bytes */
} SwiftStringView;

/* ============================================================================
 * Context Management
 * ============================================================================ */

/*
 * Create a new demangling context.
 * Contexts are reusable and reduce memory allocations when demangling
 * multiple symbols. Returns NULL on failure.
 */
SWIFT_DEMANGLE_EXPORT struct SwiftDemangleContext* swift_demangle_context_create(void);

/*
 * Destroy a demangling context and free all associated resources.
 * Any nodes obtained from this context become invalid.
 */
SWIFT_DEMANGLE_EXPORT void swift_demangle_context_destroy(struct SwiftDemangleContext* ctx);

/*
 * Clear a context, invalidating all nodes obtained from it.
 * Useful for reusing a context without destroying it.
 */
SWIFT_DEMANGLE_EXPORT void swift_demangle_context_clear(struct SwiftDemangleContext* ctx);

/* ============================================================================
 * Simple Demangling (string output)
 * ============================================================================ */

/*
 * Demangle a Swift symbol to a human-readable string.
 * Returns a newly allocated string that the caller must free with
 * swift_demangle_free_string(), or NULL on failure.
 *
 * Example:
 *   char* result = swift_demangle_symbol("$s4main5helloSSyYaKF");
 *   // result = "main.hello() async throws -> Swift.String"
 *   swift_demangle_free_string(result);
 */
SWIFT_DEMANGLE_EXPORT char* swift_demangle_symbol(const char* mangled_name);

/*
 * Free a string returned by swift_demangle_symbol() or other API functions.
 */
SWIFT_DEMANGLE_EXPORT void swift_demangle_free_string(char* str);

/* ============================================================================
 * Node Tree API (for detailed analysis)
 * ============================================================================ */

/*
 * Demangle a symbol and return the root node of the parse tree.
 * Returns NULL on failure. The returned node is owned by the context
 * and remains valid until the context is destroyed or cleared.
 */
SWIFT_DEMANGLE_EXPORT struct SwiftDemangleNode* swift_demangle_symbol_as_node(
    struct SwiftDemangleContext* ctx,
    const char* mangled_name);

/*
 * Get the kind of a node.
 */
SWIFT_DEMANGLE_EXPORT SwiftNodeKind swift_demangle_node_get_kind(struct SwiftDemangleNode* node);

/*
 * Get the kind name as a string (for debugging).
 * Returns a static string, do not free.
 */
SWIFT_DEMANGLE_EXPORT const char* swift_demangle_node_get_kind_name(struct SwiftDemangleNode* node);

/*
 * Check if a node has text content.
 */
SWIFT_DEMANGLE_EXPORT bool swift_demangle_node_has_text(struct SwiftDemangleNode* node);

/*
 * Get the text content of a node as a string view (ptr + length).
 * Returns a SwiftStringView with data=NULL and length=0 if the node has no text.
 * The returned data pointer is valid until the context is destroyed or cleared.
 * The string is NOT null-terminated; use the length field.
 */
SWIFT_DEMANGLE_EXPORT SwiftStringView swift_demangle_node_get_text(struct SwiftDemangleNode* node);

/*
 * Check if a node has an index value.
 */
SWIFT_DEMANGLE_EXPORT bool swift_demangle_node_has_index(struct SwiftDemangleNode* node);

/*
 * Get the index value of a node.
 * Returns 0 if the node has no index.
 */
SWIFT_DEMANGLE_EXPORT uint64_t swift_demangle_node_get_index(struct SwiftDemangleNode* node);

/*
 * Get the number of children of a node.
 */
SWIFT_DEMANGLE_EXPORT size_t swift_demangle_node_get_num_children(struct SwiftDemangleNode* node);

/*
 * Get a child node by index.
 * Returns NULL if index is out of bounds.
 */
SWIFT_DEMANGLE_EXPORT struct SwiftDemangleNode* swift_demangle_node_get_child(
    struct SwiftDemangleNode* node,
    size_t index);

/*
 * Convert a node subtree to a human-readable string.
 * Returns a newly allocated string that the caller must free with
 * swift_demangle_free_string(), or NULL on failure.
 */
SWIFT_DEMANGLE_EXPORT char* swift_demangle_node_to_string(struct SwiftDemangleNode* node);

/* ============================================================================
 * Convenience: Function Info Extraction
 * ============================================================================ */

typedef struct {
    char* module_name;      /* Module containing the function (caller frees) */
    char* function_name;    /* Function name (caller frees) */
    char* full_name;        /* Full demangled name (caller frees) */
    bool is_async;          /* Function is async */
    bool is_throwing;       /* Function can throw */
    bool has_typed_throws;  /* Function has typed throws */
    size_t num_parameters;  /* Number of parameters */
    char** parameter_types; /* Array of parameter type strings (caller frees each + array) */
    char* return_type;      /* Return type string (caller frees) */
} SwiftFunctionInfo;

/*
 * Extract function information from a mangled symbol.
 * Returns true on success, false if the symbol is not a function.
 * The caller must call swift_function_info_destroy() to free the contents.
 */
SWIFT_DEMANGLE_EXPORT bool swift_demangle_get_function_info(
    const char* mangled_name,
    SwiftFunctionInfo* out_info);

/*
 * Free the contents of a SwiftFunctionInfo struct.
 */
SWIFT_DEMANGLE_EXPORT void swift_function_info_destroy(SwiftFunctionInfo* info);

#ifdef __cplusplus
}
#endif

#endif /* SWIFT_DEMANGLE_H */
