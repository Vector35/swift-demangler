/*
 * swift_demangle.cpp - Implementation of the Swift demangling C API
 */

#include "swift_demangle.h"

#include "swift/Demangling/Demangle.h"

#include <cstring>
#include <string>
#include <vector>

using namespace swift::Demangle;

/* ============================================================================
 * Internal: Map Swift node kinds to our public enum
 *
 * Both enums are generated from the same DemangleNodes.def, so the numeric
 * values match. We verify this with static_asserts.
 * ============================================================================ */

// Verify a few key node kinds match (compile-time check)
static_assert(static_cast<int>(Node::Kind::Global) == SwiftNodeKind_Global, "");
static_assert(static_cast<int>(Node::Kind::Function) == SwiftNodeKind_Function, "");
static_assert(static_cast<int>(Node::Kind::Module) == SwiftNodeKind_Module, "");
static_assert(static_cast<int>(Node::Kind::Identifier) == SwiftNodeKind_Identifier, "");
static_assert(static_cast<int>(Node::Kind::AsyncAnnotation) == SwiftNodeKind_AsyncAnnotation, "");
static_assert(static_cast<int>(Node::Kind::ThrowsAnnotation) == SwiftNodeKind_ThrowsAnnotation, "");

static SwiftNodeKind mapNodeKind(Node::Kind kind) {
    // Direct cast is safe since both enums are generated from the same .def file
    return static_cast<SwiftNodeKind>(kind);
}

/* ============================================================================
 * Internal: Helper to duplicate a string
 * ============================================================================ */

static char* duplicateString(const std::string& str) {
    char* result = static_cast<char*>(malloc(str.size() + 1));
    if (result) {
        memcpy(result, str.c_str(), str.size() + 1);
    }
    return result;
}

static char* duplicateString(llvm::StringRef str) {
    char* result = static_cast<char*>(malloc(str.size() + 1));
    if (result) {
        memcpy(result, str.data(), str.size());
        result[str.size()] = '\0';
    }
    return result;
}

/* ============================================================================
 * Internal: Find node helper
 * ============================================================================ */

static NodePointer findNode(NodePointer node, Node::Kind kind, int maxDepth = 20) {
    if (!node || maxDepth <= 0) return nullptr;
    if (node->getKind() == kind) return node;
    for (auto child : *node) {
        if (auto found = findNode(child, kind, maxDepth - 1))
            return found;
    }
    return nullptr;
}

/* ============================================================================
 * Context Management
 * ============================================================================ */

struct SwiftDemangleContext {
    Context ctx;
};

struct SwiftDemangleContext* swift_demangle_context_create(void) {
    return new (std::nothrow) SwiftDemangleContext();
}

void swift_demangle_context_destroy(struct SwiftDemangleContext* ctx) {
    delete ctx;
}

void swift_demangle_context_clear(struct SwiftDemangleContext* ctx) {
    if (ctx) {
        ctx->ctx.clear();
    }
}

/* ============================================================================
 * Simple Demangling
 * ============================================================================ */

char* swift_demangle_symbol(const char* mangled_name) {
    if (!mangled_name) return nullptr;

    Context ctx;
    NodePointer node = ctx.demangleSymbolAsNode(mangled_name);
    if (!node) return nullptr;

    std::string result = nodeToString(node);
    return duplicateString(result);
}

void swift_demangle_free_string(char* str) {
    free(str);
}

/* ============================================================================
 * Node Tree API
 * ============================================================================ */

struct SwiftDemangleNode* swift_demangle_symbol_as_node(
    struct SwiftDemangleContext* ctx,
    const char* mangled_name)
{
    if (!ctx || !mangled_name) return nullptr;

    NodePointer node = ctx->ctx.demangleSymbolAsNode(mangled_name);
    return reinterpret_cast<struct SwiftDemangleNode*>(node);
}

SwiftNodeKind swift_demangle_node_get_kind(struct SwiftDemangleNode* node) {
    if (!node) return SwiftNodeKind_Global; // Return something safe
    auto* np = reinterpret_cast<NodePointer>(node);
    return mapNodeKind(np->getKind());
}

const char* swift_demangle_node_get_kind_name(struct SwiftDemangleNode* node) {
    if (!node) return "Unknown";
    auto* np = reinterpret_cast<NodePointer>(node);
    return getNodeKindString(np->getKind());
}

bool swift_demangle_node_has_text(struct SwiftDemangleNode* node) {
    if (!node) return false;
    auto* np = reinterpret_cast<NodePointer>(node);
    return np->hasText();
}

SwiftStringView swift_demangle_node_get_text(struct SwiftDemangleNode* node) {
    SwiftStringView result = { nullptr, 0 };
    if (!node) return result;
    auto* np = reinterpret_cast<NodePointer>(node);
    if (!np->hasText()) return result;

    // Return a view directly into the node's text storage.
    // The underlying StringRef points into the Context's memory arena,
    // so this pointer is valid until the context is destroyed or cleared.
    llvm::StringRef text = np->getText();
    result.data = text.data();
    result.length = text.size();
    return result;
}

bool swift_demangle_node_has_index(struct SwiftDemangleNode* node) {
    if (!node) return false;
    auto* np = reinterpret_cast<NodePointer>(node);
    return np->hasIndex();
}

uint64_t swift_demangle_node_get_index(struct SwiftDemangleNode* node) {
    if (!node) return 0;
    auto* np = reinterpret_cast<NodePointer>(node);
    return np->hasIndex() ? np->getIndex() : 0;
}

size_t swift_demangle_node_get_num_children(struct SwiftDemangleNode* node) {
    if (!node) return 0;
    auto* np = reinterpret_cast<NodePointer>(node);
    return np->getNumChildren();
}

struct SwiftDemangleNode* swift_demangle_node_get_child(
    struct SwiftDemangleNode* node,
    size_t index)
{
    if (!node) return nullptr;
    auto* np = reinterpret_cast<NodePointer>(node);
    if (index >= np->getNumChildren()) return nullptr;
    return reinterpret_cast<struct SwiftDemangleNode*>(np->getChild(index));
}

char* swift_demangle_node_to_string(struct SwiftDemangleNode* node) {
    if (!node) return nullptr;
    auto* np = reinterpret_cast<NodePointer>(node);
    std::string result = nodeToString(np);
    return duplicateString(result);
}

/* ============================================================================
 * Function Info Extraction
 * ============================================================================ */

bool swift_demangle_get_function_info(
    const char* mangled_name,
    SwiftFunctionInfo* out_info)
{
    if (!mangled_name || !out_info) return false;

    // Zero out the struct
    memset(out_info, 0, sizeof(SwiftFunctionInfo));

    Context ctx;
    NodePointer root = ctx.demangleSymbolAsNode(mangled_name);
    if (!root) return false;

    // Find the function node
    NodePointer fn = findNode(root, Node::Kind::Function);
    if (!fn) fn = findNode(root, Node::Kind::Constructor);
    if (!fn) fn = findNode(root, Node::Kind::Destructor);
    if (!fn) fn = findNode(root, Node::Kind::Getter);
    if (!fn) fn = findNode(root, Node::Kind::Setter);
    if (!fn) return false;

    // Full demangled name
    out_info->full_name = duplicateString(nodeToString(root));

    // Extract module and function name
    for (auto child : *fn) {
        if (child->getKind() == Node::Kind::Module && child->hasText()) {
            out_info->module_name = duplicateString(child->getText());
        }
        if (child->getKind() == Node::Kind::Identifier && child->hasText()) {
            out_info->function_name = duplicateString(child->getText());
        }
    }

    // Check for async/throws
    out_info->is_async = findNode(fn, Node::Kind::AsyncAnnotation) != nullptr;
    out_info->is_throwing = findNode(fn, Node::Kind::ThrowsAnnotation) != nullptr;
    out_info->has_typed_throws = findNode(fn, Node::Kind::TypedThrowsAnnotation) != nullptr;

    // Find FunctionType for parameter/return types
    if (auto fnType = findNode(fn, Node::Kind::FunctionType)) {
        // Collect parameter types
        std::vector<std::string> paramTypes;
        if (auto argTuple = findNode(fnType, Node::Kind::ArgumentTuple)) {
            // Navigate to the tuple inside
            if (auto tupleType = findNode(argTuple, Node::Kind::Tuple)) {
                for (auto param : *tupleType) {
                    paramTypes.push_back(nodeToString(param));
                }
            } else {
                // Single parameter or empty
                for (auto child : *argTuple) {
                    if (child->getKind() == Node::Kind::Type) {
                        paramTypes.push_back(nodeToString(child));
                    }
                }
            }
        }

        out_info->num_parameters = paramTypes.size();
        if (!paramTypes.empty()) {
            out_info->parameter_types = static_cast<char**>(
                malloc(paramTypes.size() * sizeof(char*)));
            for (size_t i = 0; i < paramTypes.size(); i++) {
                out_info->parameter_types[i] = duplicateString(paramTypes[i]);
            }
        }

        // Return type
        if (auto retType = findNode(fnType, Node::Kind::ReturnType)) {
            out_info->return_type = duplicateString(nodeToString(retType));
        }
    }

    return true;
}

void swift_function_info_destroy(SwiftFunctionInfo* info) {
    if (!info) return;

    free(info->module_name);
    free(info->function_name);
    free(info->full_name);
    free(info->return_type);

    if (info->parameter_types) {
        for (size_t i = 0; i < info->num_parameters; i++) {
            free(info->parameter_types[i]);
        }
        free(info->parameter_types);
    }

    memset(info, 0, sizeof(SwiftFunctionInfo));
}
