//! Raw FFI bindings to the swift_demangler C library.

use std::os::raw::{c_char, c_int};

#[repr(C)]
pub struct SwiftDemangleContext {
    _private: [u8; 0],
}

#[repr(C)]
pub struct SwiftDemangleNode {
    _private: [u8; 0],
}

/// A non-owning view of a string (ptr + length).
/// The data pointer is NOT null-terminated; use the length field.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SwiftStringView {
    pub data: *const c_char,
    pub length: usize,
}

#[repr(C)]
pub struct SwiftFunctionInfo {
    pub module_name: *mut c_char,
    pub function_name: *mut c_char,
    pub full_name: *mut c_char,
    pub is_async: bool,
    pub is_throwing: bool,
    pub has_typed_throws: bool,
    pub num_parameters: usize,
    pub parameter_types: *mut *mut c_char,
    pub return_type: *mut c_char,
}

unsafe extern "C" {
    // Context management
    pub fn swift_demangle_context_create() -> *mut SwiftDemangleContext;
    pub fn swift_demangle_context_destroy(ctx: *mut SwiftDemangleContext);
    pub fn swift_demangle_context_clear(ctx: *mut SwiftDemangleContext);

    // Simple demangling
    pub fn swift_demangle_symbol(mangled_name: *const c_char) -> *mut c_char;
    pub fn swift_demangle_free_string(s: *mut c_char);

    // Node tree API
    pub fn swift_demangle_symbol_as_node(
        ctx: *mut SwiftDemangleContext,
        mangled_name: *const c_char,
    ) -> *mut SwiftDemangleNode;

    pub fn swift_demangle_node_get_kind(node: *mut SwiftDemangleNode) -> c_int;
    pub fn swift_demangle_node_has_text(node: *mut SwiftDemangleNode) -> bool;
    pub fn swift_demangle_node_get_text(node: *mut SwiftDemangleNode) -> SwiftStringView;
    pub fn swift_demangle_node_has_index(node: *mut SwiftDemangleNode) -> bool;
    pub fn swift_demangle_node_get_index(node: *mut SwiftDemangleNode) -> u64;
    pub fn swift_demangle_node_get_num_children(node: *mut SwiftDemangleNode) -> usize;
    pub fn swift_demangle_node_get_child(
        node: *mut SwiftDemangleNode,
        index: usize,
    ) -> *mut SwiftDemangleNode;
    pub fn swift_demangle_node_to_string(node: *mut SwiftDemangleNode) -> *mut c_char;

    // Function info
    pub fn swift_demangle_get_function_info(
        mangled_name: *const c_char,
        out_info: *mut SwiftFunctionInfo,
    ) -> bool;
    pub fn swift_function_info_destroy(info: *mut SwiftFunctionInfo);
}
