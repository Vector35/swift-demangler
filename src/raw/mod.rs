//! Low-level Swift symbol demangling API.
//!
//! This module provides direct access to the Swift demangling infrastructure,
//! including the raw node tree and FFI bindings.
//!
//! # Examples
//!
//! Simple demangling:
//! ```
//! use swift_demangler::raw::demangle;
//!
//! let demangled = demangle("$s4main5helloSSyYaKF").unwrap();
//! assert_eq!(demangled, "main.hello() async throws -> Swift.String");
//! ```
//!
//! Extracting function information:
//! ```
//! use swift_demangler::raw::FunctionInfo;
//!
//! if let Some(info) = FunctionInfo::parse("$s4main5helloSSyYaKF") {
//!     assert_eq!(info.module_name(), Some("main"));
//!     assert_eq!(info.function_name(), Some("hello"));
//!     assert!(info.is_async());
//!     assert!(info.is_throwing());
//! }
//! ```
//!
//! Traversing the node tree:
//! ```
//! use swift_demangler::raw::{Context, Node, NodeKind};
//!
//! let ctx = Context::new();
//! if let Some(root) = Node::parse(&ctx, "$s4main5helloSSyYaKF") {
//!     println!("Root kind: {:?}", root.kind());
//!     for child in root.children() {
//!         println!("  Child: {:?}", child.kind());
//!     }
//!
//!     // Find specific node types
//!     if let Some(module) = root.descendants().find(|n| n.kind() == NodeKind::Module) {
//!         println!("Module: {:?}", module.text());
//!     }
//! }
//! ```

mod ffi;
mod node_kinds;

use std::ffi::{CStr, CString};
use std::fmt;
use std::marker::PhantomData;
use std::ptr::NonNull;

pub use node_kinds::NodeKind;

/// Demangle a Swift symbol to a human-readable string.
///
/// Returns `None` if the symbol could not be demangled.
pub fn demangle(mangled: &str) -> Option<String> {
    let c_mangled = CString::new(mangled).ok()?;
    unsafe {
        let result = ffi::swift_demangle_symbol(c_mangled.as_ptr());
        if result.is_null() {
            return None;
        }
        let s = CStr::from_ptr(result).to_string_lossy().into_owned();
        ffi::swift_demangle_free_string(result);
        Some(s)
    }
}

/// A demangling context that can be reused for multiple symbols.
///
/// Using a context reduces memory allocations when demangling many symbols.
/// Nodes returned from this context are valid until the context is dropped or cleared.
pub struct Context {
    ptr: NonNull<ffi::SwiftDemangleContext>,
}

impl Context {
    /// Create a new demangling context.
    pub fn new() -> Self {
        let ptr = unsafe { ffi::swift_demangle_context_create() };
        Context {
            ptr: NonNull::new(ptr).expect("Failed to create demangling context"),
        }
    }

    /// Clear the context, invalidating all nodes obtained from it.
    pub fn clear(&mut self) {
        unsafe {
            ffi::swift_demangle_context_clear(self.ptr.as_ptr());
        }
    }
}

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        unsafe {
            ffi::swift_demangle_context_destroy(self.ptr.as_ptr());
        }
    }
}

// Note: Context is !Send and !Sync due to the raw pointer, which is correct
// since the underlying C++ context is not thread-safe.

/// A node in the demangled symbol tree.
///
/// Nodes borrow from the `Context` that created them and are valid
/// until the context is dropped or cleared.
#[derive(Clone, Copy)]
pub struct Node<'ctx> {
    ptr: NonNull<ffi::SwiftDemangleNode>,
    _lifetime: PhantomData<&'ctx Context>,
}

impl<'ctx> Node<'ctx> {
    /// Parse a mangled symbol and return the root node of the parse tree.
    ///
    /// The returned node borrows from the context and is valid until
    /// the context is dropped or `clear()` is called.
    pub fn parse(ctx: &'ctx Context, mangled: &str) -> Option<Node<'ctx>> {
        let c_mangled = CString::new(mangled).ok()?;
        unsafe {
            let node = ffi::swift_demangle_symbol_as_node(ctx.ptr.as_ptr(), c_mangled.as_ptr());
            NonNull::new(node).map(|ptr| Node {
                ptr,
                _lifetime: PhantomData,
            })
        }
    }

    /// Get the kind of this node.
    pub fn kind(&self) -> NodeKind {
        let raw = unsafe { ffi::swift_demangle_node_get_kind(self.ptr.as_ptr()) };
        NodeKind::from_raw(raw).expect("Invalid node kind")
    }

    /// Check if this node has text content.
    pub fn has_text(&self) -> bool {
        unsafe { ffi::swift_demangle_node_has_text(self.ptr.as_ptr()) }
    }

    /// Get the text content of this node, if any.
    pub fn text(&self) -> Option<&'ctx str> {
        if !self.has_text() {
            return None;
        }
        unsafe {
            let view = ffi::swift_demangle_node_get_text(self.ptr.as_ptr());
            if view.data.is_null() || view.length == 0 {
                None
            } else {
                // Create a str slice from the raw pointer and length.
                // The data points into the Context's memory arena and is valid
                // for the 'ctx lifetime.
                let bytes = std::slice::from_raw_parts(view.data as *const u8, view.length);
                std::str::from_utf8(bytes).ok()
            }
        }
    }

    /// Check if this node has an index value.
    pub fn has_index(&self) -> bool {
        unsafe { ffi::swift_demangle_node_has_index(self.ptr.as_ptr()) }
    }

    /// Get the index value of this node, if any.
    pub fn index(&self) -> Option<u64> {
        if self.has_index() {
            Some(unsafe { ffi::swift_demangle_node_get_index(self.ptr.as_ptr()) })
        } else {
            None
        }
    }

    /// Get the number of children.
    pub fn num_children(&self) -> usize {
        unsafe { ffi::swift_demangle_node_get_num_children(self.ptr.as_ptr()) }
    }

    /// Get a child by index.
    pub fn child(&self, index: usize) -> Option<Node<'ctx>> {
        unsafe {
            let child = ffi::swift_demangle_node_get_child(self.ptr.as_ptr(), index);
            NonNull::new(child).map(|ptr| Node {
                ptr,
                _lifetime: PhantomData,
            })
        }
    }

    /// Iterate over all children.
    pub fn children(&self) -> Children<'ctx> {
        Children {
            node: *self,
            index: 0,
            len: self.num_children(),
        }
    }

    /// Iterate over all descendants (depth-first).
    pub fn descendants(&self) -> Descendants<'ctx> {
        Descendants {
            stack: self.children().collect(),
        }
    }
}

impl fmt::Debug for Node<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let children: Vec<_> = self.children().collect();
        if f.alternate() && children.is_empty() {
            #[allow(clippy::recursive_format_impl)]
            return write!(f, "{self:?}");
        }

        let mut s = f.debug_struct("Node");
        s.field("kind", &self.kind());
        if let Some(text) = self.text() {
            s.field("text", &text);
        }
        if let Some(index) = self.index() {
            s.field("index", &index);
        }
        if !children.is_empty() {
            s.field("children", &children);
        }
        s.finish()
    }
}

impl fmt::Display for Node<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        unsafe {
            let s = ffi::swift_demangle_node_to_string(self.ptr.as_ptr());
            if s.is_null() {
                Ok(())
            } else {
                let result = CStr::from_ptr(s).to_string_lossy();
                let r = f.write_str(&result);
                ffi::swift_demangle_free_string(s);
                r
            }
        }
    }
}

/// Iterator over a node's children.
pub struct Children<'ctx> {
    node: Node<'ctx>,
    index: usize,
    len: usize,
}

impl<'ctx> Iterator for Children<'ctx> {
    type Item = Node<'ctx>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.len {
            return None;
        }
        let child = self.node.child(self.index);
        self.index += 1;
        child
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.len - self.index;
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for Children<'_> {}

/// Iterator over all descendants of a node (depth-first).
pub struct Descendants<'ctx> {
    stack: Vec<Node<'ctx>>,
}

impl<'ctx> Iterator for Descendants<'ctx> {
    type Item = Node<'ctx>;

    fn next(&mut self) -> Option<Self::Item> {
        let node = self.stack.pop()?;
        // Push children in reverse order so we visit them left-to-right
        for i in (0..node.num_children()).rev() {
            if let Some(child) = node.child(i) {
                self.stack.push(child);
            }
        }
        Some(node)
    }
}

/// Extracted information about a Swift function symbol.
#[derive(Debug, Clone)]
pub struct FunctionInfo {
    module_name: Option<String>,
    function_name: Option<String>,
    full_name: Option<String>,
    is_async: bool,
    is_throwing: bool,
    has_typed_throws: bool,
    parameter_types: Vec<String>,
    return_type: Option<String>,
}

impl FunctionInfo {
    /// Parse a mangled symbol and extract function information.
    ///
    /// Returns `None` if the symbol is not a function.
    pub fn parse(mangled: &str) -> Option<Self> {
        let c_mangled = CString::new(mangled).ok()?;
        unsafe {
            let mut info = std::mem::zeroed::<ffi::SwiftFunctionInfo>();
            if !ffi::swift_demangle_get_function_info(c_mangled.as_ptr(), &mut info) {
                return None;
            }

            let result = FunctionInfo {
                module_name: ptr_to_option_string(info.module_name),
                function_name: ptr_to_option_string(info.function_name),
                full_name: ptr_to_option_string(info.full_name),
                is_async: info.is_async,
                is_throwing: info.is_throwing,
                has_typed_throws: info.has_typed_throws,
                parameter_types: (0..info.num_parameters)
                    .filter_map(|i| ptr_to_option_string(*info.parameter_types.add(i)))
                    .collect(),
                return_type: ptr_to_option_string(info.return_type),
            };

            ffi::swift_function_info_destroy(&mut info);
            Some(result)
        }
    }

    /// The module containing the function.
    pub fn module_name(&self) -> Option<&str> {
        self.module_name.as_deref()
    }

    /// The function name.
    pub fn function_name(&self) -> Option<&str> {
        self.function_name.as_deref()
    }

    /// The full demangled name.
    pub fn full_name(&self) -> Option<&str> {
        self.full_name.as_deref()
    }

    /// Whether the function is async.
    pub fn is_async(&self) -> bool {
        self.is_async
    }

    /// Whether the function can throw.
    pub fn is_throwing(&self) -> bool {
        self.is_throwing
    }

    /// Whether the function has typed throws.
    pub fn has_typed_throws(&self) -> bool {
        self.has_typed_throws
    }

    /// The parameter types.
    pub fn parameter_types(&self) -> &[String] {
        &self.parameter_types
    }

    /// The return type.
    pub fn return_type(&self) -> Option<&str> {
        self.return_type.as_deref()
    }
}

unsafe fn ptr_to_option_string(ptr: *mut std::os::raw::c_char) -> Option<String> {
    if ptr.is_null() {
        None
    } else {
        Some(
            unsafe { CStr::from_ptr(ptr) }
                .to_string_lossy()
                .into_owned(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_SYMBOL: &str = "$s4main5helloSSyYaKF";

    #[test]
    fn test_demangle() {
        let result = demangle(TEST_SYMBOL).unwrap();
        assert_eq!(result, "main.hello() async throws -> Swift.String");
    }

    #[test]
    fn test_context_and_nodes() {
        let ctx = Context::new();
        let root = Node::parse(&ctx, TEST_SYMBOL).unwrap();

        assert_eq!(root.kind(), NodeKind::Global);
        assert_eq!(root.num_children(), 1);

        let func = root.child(0).unwrap();
        assert_eq!(func.kind(), NodeKind::Function);
    }

    #[test]
    fn test_children_iterator() {
        let ctx = Context::new();
        let root = Node::parse(&ctx, TEST_SYMBOL).unwrap();
        let func = root.child(0).unwrap();

        let children: Vec<_> = func.children().collect();
        assert!(!children.is_empty());

        // Should have Module, Identifier, Type children
        let kinds: Vec<_> = children.iter().map(|n| n.kind()).collect();
        assert!(kinds.contains(&NodeKind::Module));
        assert!(kinds.contains(&NodeKind::Identifier));
    }

    #[test]
    fn test_descendants() {
        let ctx = Context::new();
        let root = Node::parse(&ctx, TEST_SYMBOL).unwrap();

        let module = root
            .descendants()
            .find(|n| n.kind() == NodeKind::Module)
            .unwrap();
        assert_eq!(module.text(), Some("main"));

        let identifier = root
            .descendants()
            .find(|n| n.kind() == NodeKind::Identifier)
            .unwrap();
        assert_eq!(identifier.text(), Some("hello"));

        // Can also collect all of a kind
        let all_identifiers: Vec<_> = root
            .descendants()
            .filter(|n| n.kind() == NodeKind::Identifier)
            .collect();
        assert!(!all_identifiers.is_empty());
    }

    #[test]
    fn test_function_info() {
        let info = FunctionInfo::parse(TEST_SYMBOL).unwrap();

        assert_eq!(info.module_name(), Some("main"));
        assert_eq!(info.function_name(), Some("hello"));
        assert!(info.is_async());
        assert!(info.is_throwing());
        assert!(!info.has_typed_throws());
        assert_eq!(info.return_type(), Some("Swift.String"));
    }

    #[test]
    fn test_invalid_symbol() {
        assert!(demangle("not_a_swift_symbol").is_none());
        assert!(FunctionInfo::parse("not_a_swift_symbol").is_none());
    }
}
