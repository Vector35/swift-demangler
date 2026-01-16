/*
 * main.cpp - CLI tool demonstrating the swift_demangle library
 *
 * This file uses ONLY the public C API from swift_demangle.h.
 * No Swift or LLVM headers are included.
 */

#include "swift_demangle.h"
#include <cstdio>
#include <cstring>

static void print_node_tree(struct SwiftDemangleNode* node, int indent) {
    if (!node) return;

    for (int i = 0; i < indent; i++) printf("  ");
    printf("%s", swift_demangle_node_get_kind_name(node));

    if (swift_demangle_node_has_text(node)) {
        SwiftStringView text = swift_demangle_node_get_text(node);
        printf(" = \"%.*s\"", (int)text.length, text.data);
    } else if (swift_demangle_node_has_index(node)) {
        printf(" = %llu", (unsigned long long)swift_demangle_node_get_index(node));
    }
    printf("\n");

    size_t num_children = swift_demangle_node_get_num_children(node);
    for (size_t i = 0; i < num_children; i++) {
        print_node_tree(swift_demangle_node_get_child(node, i), indent + 1);
    }
}

static void print_usage(const char* prog_name) {
    printf("Swift Symbol Demangler\n");
    printf("Usage: %s [options] <mangled-symbol>\n", prog_name);
    printf("\nOptions:\n");
    printf("  -t, --tree      Print the full node tree\n");
    printf("  -f, --function  Show function details (async, throws, params)\n");
    printf("  -h, --help      Show this help message\n");
    printf("\nExamples:\n");
    printf("  %s '$s4main5helloSSyYaKF'\n", prog_name);
    printf("  %s --tree '$s4main5helloSSyYaKF'\n", prog_name);
    printf("  %s --function '$s4main5helloSSyYaKF'\n", prog_name);
}

int main(int argc, char** argv) {
    bool show_tree = false;
    bool show_function = false;
    const char* mangled = nullptr;

    for (int i = 1; i < argc; i++) {
        if (strcmp(argv[i], "-t") == 0 || strcmp(argv[i], "--tree") == 0) {
            show_tree = true;
        } else if (strcmp(argv[i], "-f") == 0 || strcmp(argv[i], "--function") == 0) {
            show_function = true;
        } else if (strcmp(argv[i], "-h") == 0 || strcmp(argv[i], "--help") == 0) {
            print_usage(argv[0]);
            return 0;
        } else if (argv[i][0] != '-') {
            mangled = argv[i];
        }
    }

    if (!mangled) {
        print_usage(argv[0]);
        return 1;
    }

    // Simple demangling
    char* demangled = swift_demangle_symbol(mangled);
    if (!demangled) {
        printf("Failed to demangle: %s\n", mangled);
        return 1;
    }
    printf("Demangled: %s\n", demangled);
    swift_demangle_free_string(demangled);

    // Node tree view
    if (show_tree) {
        struct SwiftDemangleContext* ctx = swift_demangle_context_create();
        if (ctx) {
            struct SwiftDemangleNode* root = swift_demangle_symbol_as_node(ctx, mangled);
            if (root) {
                printf("\nNode tree:\n");
                print_node_tree(root, 0);
            }
            swift_demangle_context_destroy(ctx);
        }
    }

    // Function info
    if (show_function) {
        SwiftFunctionInfo info;
        if (swift_demangle_get_function_info(mangled, &info)) {
            printf("\nFunction Info:\n");
            printf("  Module: %s\n", info.module_name ? info.module_name : "(unknown)");
            printf("  Name: %s\n", info.function_name ? info.function_name : "(unknown)");
            printf("  Async: %s\n", info.is_async ? "yes" : "no");
            printf("  Throws: %s\n", info.is_throwing ? "yes" : "no");
            printf("  Typed Throws: %s\n", info.has_typed_throws ? "yes" : "no");
            printf("  Parameters: %zu\n", info.num_parameters);
            for (size_t i = 0; i < info.num_parameters; i++) {
                printf("    [%zu]: %s\n", i, info.parameter_types[i]);
            }
            printf("  Return Type: %s\n", info.return_type ? info.return_type : "(none)");
            swift_function_info_destroy(&info);
        } else {
            printf("\n(Not a function symbol)\n");
        }
    }

    return 0;
}
