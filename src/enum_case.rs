//! Enum case symbol representation.
//!
//! This module provides types for representing Swift enum case constructors.

use crate::helpers::{HasGenericSignature, NodeExt};
use crate::raw::{Node, NodeKind};
use crate::types::{FunctionType, GenericSignature, TypeRef};

/// A Swift enum case constructor symbol.
#[derive(Clone, Copy)]
pub struct EnumCase<'ctx> {
    raw: Node<'ctx>,
}

impl<'ctx> EnumCase<'ctx> {
    /// Create an EnumCase from a raw node.
    pub fn new(raw: Node<'ctx>) -> Self {
        Self { raw }
    }

    /// Get the underlying raw node.
    pub fn raw(&self) -> Node<'ctx> {
        self.raw
    }

    /// Get the name of the enum case.
    pub fn case_name(&self) -> Option<&'ctx str> {
        self.function_node().and_then(|n| n.find_identifier())
    }

    /// Get the containing enum type name.
    pub fn containing_type(&self) -> Option<&'ctx str> {
        if let Some(func) = self.function_node() {
            for child in func.children() {
                if child.kind() == NodeKind::Enum {
                    for inner in child.children() {
                        if inner.kind() == NodeKind::Identifier {
                            return inner.text();
                        }
                    }
                }
            }
        }
        None
    }

    /// Get the full containing type as a TypeRef.
    pub fn containing_type_ref(&self) -> Option<TypeRef<'ctx>> {
        if let Some(func) = self.function_node() {
            for child in func.children() {
                if child.kind() == NodeKind::Enum {
                    return Some(TypeRef::new(child));
                }
            }
        }
        None
    }

    /// Get the module containing this enum case.
    pub fn module(&self) -> Option<&'ctx str> {
        // Search for Module in the enum type
        if let Some(func) = self.function_node() {
            for child in func.children() {
                if child.kind() == NodeKind::Enum {
                    for inner in child.descendants() {
                        if inner.kind() == NodeKind::Module {
                            return inner.text();
                        }
                    }
                }
            }
        }
        None
    }

    /// Get the signature of the enum case constructor.
    ///
    /// For cases with associated values, this includes the parameter types.
    pub fn signature(&self) -> Option<FunctionType<'ctx>> {
        self.function_node().and_then(|n| n.find_function_type())
    }

    /// Check if this enum case has associated values.
    pub fn has_associated_values(&self) -> bool {
        if let Some(sig) = self.signature() {
            let params = sig.parameters();
            // Cases without associated values just have a metatype parameter
            // Cases with associated values have additional parameters
            if params.len() > 1 {
                return true;
            }
            // Check if the single parameter is not just a metatype
            if let Some(first) = params.first() {
                return !matches!(first.type_ref.kind(), crate::types::TypeKind::Metatype(_));
            }
        }
        false
    }

    /// Get the associated value types if any.
    pub fn associated_values(&self) -> Vec<TypeRef<'ctx>> {
        if let Some(sig) = self.signature() {
            let params = sig.parameters();
            // Filter out the metatype parameter
            return params
                .into_iter()
                .filter(|p| !matches!(p.type_ref.kind(), crate::types::TypeKind::Metatype(_)))
                .map(|p| p.type_ref)
                .collect();
        }
        Vec::new()
    }

    fn function_node(&self) -> Option<Node<'ctx>> {
        // EnumCase contains a Function child
        self.raw
            .children()
            .find(|&child| child.kind() == NodeKind::Function)
    }
}

impl<'ctx> HasGenericSignature<'ctx> for EnumCase<'ctx> {
    fn generic_signature(&self) -> Option<GenericSignature<'ctx>> {
        self.function_node()
            .and_then(|n| n.find_generic_signature())
    }
}

impl std::fmt::Debug for EnumCase<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = f.debug_struct("EnumCase");
        s.field("case_name", &self.case_name())
            .field("containing_type", &self.containing_type())
            .field("module", &self.module())
            .field("has_associated_values", &self.has_associated_values())
            .field("is_generic", &self.is_generic());
        if let Some(sig) = self.signature() {
            s.field("signature", &sig);
        }
        let requirements = self.generic_requirements();
        if !requirements.is_empty() {
            s.field("generic_requirements", &requirements);
        }
        s.finish()
    }
}

impl std::fmt::Display for EnumCase<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.raw)
    }
}

#[cfg(test)]
mod tests {
    use crate::Symbol;
    use crate::raw::Context;

    #[test]
    fn test_enum_case() {
        let ctx = Context::new();
        // enum case for Swift.Mirror.DisplayStyle.enum
        let symbol = Symbol::parse(&ctx, "$ss6MirrorV12DisplayStyleO4enumyA2DmFWC").unwrap();
        assert!(symbol.is_enum_case());
        if let Symbol::EnumCase(ec) = symbol {
            assert_eq!(ec.case_name(), Some("enum"));
            assert_eq!(ec.containing_type(), Some("DisplayStyle"));
            assert_eq!(ec.module(), Some("Swift"));
            assert!(!ec.has_associated_values());
        } else {
            panic!("Expected enum case");
        }
    }
}
