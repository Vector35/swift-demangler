//! Function symbol representation.
//!
//! This module provides the `Function` struct for representing Swift function symbols.

use crate::context::{SymbolContext, extract_context};
use crate::helpers::{
    HasExtensionContext, HasFunctionSignature, HasGenericSignature, HasModule, NodeExt,
};
use crate::raw::Node;
use crate::types::{FunctionType, GenericSignature, TypeRef};

/// A Swift function symbol.
#[derive(Clone, Copy)]
pub struct Function<'ctx> {
    raw: Node<'ctx>,
    is_static: bool,
}

impl<'ctx> Function<'ctx> {
    /// Create a Function from a raw node.
    pub fn new(raw: Node<'ctx>) -> Self {
        Self {
            raw,
            is_static: false,
        }
    }

    /// Create a Function from a raw node, marking it as static.
    pub fn new_static(raw: Node<'ctx>) -> Self {
        Self {
            raw,
            is_static: true,
        }
    }

    /// Get the underlying raw node.
    pub fn raw(&self) -> Node<'ctx> {
        self.raw
    }

    /// Get the context (location) where this function is defined.
    pub fn context(&self) -> SymbolContext<'ctx> {
        extract_context(self.raw)
    }

    /// Get the name of this function.
    pub fn name(&self) -> Option<&'ctx str> {
        self.raw.find_identifier_extended()
    }

    /// Get the argument labels for this function.
    ///
    /// Returns a vector where each element is `Some(label)` for labeled parameters
    /// and `None` for unlabeled parameters (using `_`).
    pub fn labels(&self) -> Vec<Option<&'ctx str>> {
        self.raw.extract_labels()
    }

    /// Get the return type of this function.
    pub fn return_type(&self) -> Option<TypeRef<'ctx>> {
        self.signature().and_then(|s| s.return_type())
    }

    /// Check if this is a method (defined in a type context).
    pub fn is_method(&self) -> bool {
        self.raw.has_type_context()
    }

    /// Check if this is a static/class method.
    pub fn is_static(&self) -> bool {
        self.is_static
    }

    /// Get the containing type name if this is a method.
    pub fn containing_type(&self) -> Option<&'ctx str> {
        self.raw.find_containing_type()
    }

    /// Check if the containing type is a class (reference type).
    pub fn containing_type_is_class(&self) -> bool {
        self.raw.containing_type_is_class()
    }

    /// Check if the containing type is a protocol.
    pub fn containing_type_is_protocol(&self) -> bool {
        self.raw.containing_type_is_protocol()
    }

    /// Get the full name with labels (e.g., "foo(bar:baz:)").
    pub fn full_name(&self) -> String {
        let name = self.name().unwrap_or("");
        let labels = self.labels();

        if labels.is_empty() {
            // Check if there are parameters from the signature
            if let Some(sig) = self.signature() {
                let params = sig.parameters();
                if params.is_empty() {
                    format!("{name}()")
                } else {
                    let param_labels: Vec<String> = params
                        .iter()
                        .map(|p| {
                            p.label
                                .map(|l| format!("{l}:"))
                                .unwrap_or_else(|| "_:".to_string())
                        })
                        .collect();
                    format!("{}({})", name, param_labels.join(""))
                }
            } else {
                format!("{name}()")
            }
        } else {
            let label_strs: Vec<String> = labels
                .iter()
                .map(|l| {
                    l.map(|s| format!("{s}:"))
                        .unwrap_or_else(|| "_:".to_string())
                })
                .collect();
            format!("{}({})", name, label_strs.join(""))
        }
    }
}

impl<'ctx> HasGenericSignature<'ctx> for Function<'ctx> {
    fn generic_signature(&self) -> Option<GenericSignature<'ctx>> {
        self.raw.find_generic_signature()
    }
}

impl<'ctx> HasFunctionSignature<'ctx> for Function<'ctx> {
    fn signature(&self) -> Option<FunctionType<'ctx>> {
        self.raw.find_function_type()
    }
}

impl<'ctx> HasExtensionContext<'ctx> for Function<'ctx> {
    fn raw(&self) -> Node<'ctx> {
        self.raw
    }
}

impl<'ctx> HasModule<'ctx> for Function<'ctx> {
    fn module(&self) -> Option<&'ctx str> {
        self.raw.find_module()
    }
}

impl std::fmt::Debug for Function<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = f.debug_struct("Function");
        s.field("module", &self.module())
            .field("containing_type", &self.containing_type())
            .field("name", &self.name())
            .field("labels", &self.labels())
            .field("is_method", &self.is_method())
            .field("is_static", &self.is_static())
            .field("is_async", &self.is_async())
            .field("is_throwing", &self.is_throwing())
            .field("is_extension", &self.is_extension())
            .field("is_generic", &self.is_generic());
        if let Some(sig) = self.signature() {
            s.field("signature", &sig);
        }
        if self.is_extension() {
            s.field("extension_module", &self.extension_module());
            let ext_requirements = self.extension_generic_requirements();
            if !ext_requirements.is_empty() {
                s.field("extension_generic_requirements", &ext_requirements);
            }
        }
        let requirements = self.generic_requirements();
        if !requirements.is_empty() {
            s.field("generic_requirements", &requirements);
        }
        s.finish()
    }
}

impl std::fmt::Display for Function<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.raw)
    }
}

#[cfg(test)]
mod tests {
    use crate::helpers::{HasFunctionSignature, HasModule};
    use crate::raw::Context;
    use crate::symbol::Symbol;

    #[test]
    fn test_simple_function() {
        let ctx = Context::new();
        let symbol = Symbol::parse(&ctx, "$s4main5helloSSyYaKF").unwrap();
        if let Symbol::Function(func) = symbol {
            assert_eq!(func.name(), Some("hello"));
            assert_eq!(func.module(), Some("main"));
            assert!(func.is_async());
            assert!(func.is_throwing());
            assert!(!func.is_method());
        } else {
            panic!("Expected function");
        }
    }

    #[test]
    fn test_method() {
        let ctx = Context::new();
        // foo.bar.bas(zim: foo.zim) -> ()
        let symbol = Symbol::parse(&ctx, "_TFC3foo3bar3basfT3zimCS_3zim_T_").unwrap();
        if let Symbol::Function(func) = symbol {
            assert_eq!(func.name(), Some("bas"));
            assert_eq!(func.module(), Some("foo"));
            assert!(func.is_method());
            assert_eq!(func.containing_type(), Some("bar"));
        } else {
            panic!("Expected function");
        }
    }

    #[test]
    fn test_function_signature() {
        let ctx = Context::new();
        let symbol = Symbol::parse(&ctx, "$s4main5helloSSyYaKF").unwrap();
        if let Symbol::Function(func) = symbol {
            let sig = func.signature().expect("Expected signature");
            assert!(sig.is_async());
            assert!(sig.is_throwing());
            assert!(sig.return_type().is_some());
        } else {
            panic!("Expected function");
        }
    }

    #[test]
    fn test_function_display() {
        let ctx = Context::new();
        let symbol = Symbol::parse(&ctx, "$s4main5helloSSyYaKF").unwrap();
        if let Symbol::Function(func) = symbol {
            let display = func.to_string();
            assert!(display.contains("hello"));
        } else {
            panic!("Expected function");
        }
    }
}
