/*
 * gen_node_kinds.c - Generates swift_node_kinds.h using the C preprocessor
 *
 * Compile with: cc -I<vendor>/include -o gen_node_kinds gen_node_kinds.c
 * Run: ./gen_node_kinds > include/swift_node_kinds.h
 *
 * This approach lets the C preprocessor handle all #include directives
 * in DemangleNodes.def naturally, avoiding manual parsing.
 */

#include <stdio.h>

/* Define the macros that DemangleNodes.def expects */
#define NODE(ID) printf("    SwiftNodeKind_%s,\n", #ID);
#define CONTEXT_NODE(ID) NODE(ID)

/* ReferenceStorage.def uses these macros - map them to NODE */
#define NEVER_LOADABLE_CHECKED_REF_STORAGE(Name, ...) NODE(Name)
#define SOMETIMES_LOADABLE_CHECKED_REF_STORAGE(Name, ...) NODE(Name)
#define ALWAYS_LOADABLE_CHECKED_REF_STORAGE(Name, ...) NODE(Name)
#define UNCHECKED_REF_STORAGE(Name, ...) NODE(Name)

int main(void) {
    printf("/*\n");
    printf(" * swift_node_kinds.h - Generated from DemangleNodes.def\n");
    printf(" * DO NOT EDIT - regenerate with scripts/generate-node-kinds.sh\n");
    printf(" */\n");
    printf("\n");
    printf("#ifndef SWIFT_NODE_KINDS_H\n");
    printf("#define SWIFT_NODE_KINDS_H\n");
    printf("\n");
    printf("#ifdef __cplusplus\n");
    printf("extern \"C\" {\n");
    printf("#endif\n");
    printf("\n");
    printf("/* Swift demangling node kinds */\n");
    printf("typedef enum {\n");

    /* Include the .def file - preprocessor expands NODE() calls to printf */
#include "swift/Demangling/DemangleNodes.def"

    printf("    SwiftNodeKind_COUNT\n");
    printf("} SwiftNodeKind;\n");
    printf("\n");
    printf("#ifdef __cplusplus\n");
    printf("}\n");
    printf("#endif\n");
    printf("\n");
    printf("#endif /* SWIFT_NODE_KINDS_H */\n");

    return 0;
}
