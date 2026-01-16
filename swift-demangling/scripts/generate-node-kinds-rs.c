/*
 * generate-node-kinds-rs.c - Generates node_kinds.rs using the C preprocessor
 */

#include <stdio.h>

static int counter = 0;

#define NODE(ID) printf("    %s = %d,\n", #ID, counter++);
#define CONTEXT_NODE(ID) NODE(ID)

#define NEVER_LOADABLE_CHECKED_REF_STORAGE(Name, ...) NODE(Name)
#define SOMETIMES_LOADABLE_CHECKED_REF_STORAGE(Name, ...) NODE(Name)
#define ALWAYS_LOADABLE_CHECKED_REF_STORAGE(Name, ...) NODE(Name)
#define UNCHECKED_REF_STORAGE(Name, ...) NODE(Name)

int main(void) {
    printf("//! Swift demangling node kinds.\n");
    printf("//!\n");
    printf("//! Generated from DemangleNodes.def - do not edit manually.\n");
    printf("\n");
    printf("#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]\n");
    printf("#[repr(i32)]\n");
    printf("#[non_exhaustive]\n");
    printf("pub enum NodeKind {\n");

#include "swift/Demangling/DemangleNodes.def"

    printf("}\n");
    printf("\n");
    printf("impl NodeKind {\n");
    printf("    /// Convert from the raw integer value returned by the C API.\n");
    printf("    pub fn from_raw(value: i32) -> Option<Self> {\n");
    printf("        if (0..%d).contains(&value) {\n", counter);
    printf("            // SAFETY: We just verified value is in range of valid enum discriminants\n");
    printf("            Some(unsafe { std::mem::transmute::<i32, NodeKind>(value) })\n");
    printf("        } else {\n");
    printf("            None\n");
    printf("        }\n");
    printf("    }\n");
    printf("\n");
    printf("    /// Get the raw integer value.\n");
    printf("    pub fn as_raw(self) -> i32 {\n");
    printf("        self as i32\n");
    printf("    }\n");
    printf("}\n");

    return 0;
}
