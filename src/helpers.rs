//! Extension traits for [`Node`] and [`NodeKind`] with shared helper methods.
//!
//! This module provides:
//! - [`NodeKindExt`] trait that extends [`NodeKind`] with classification predicates
//! - [`NodeExt`] trait that extends [`Node`] with utility methods

use crate::raw::{Node, NodeKind};
use crate::types::{FunctionType, GenericRequirement, GenericSignature, TypeRef};

/// Extension trait adding classification methods to [`NodeKind`].
pub(crate) trait NodeKindExt {
    /// Check if this kind represents a function type.
    fn is_function_type(&self) -> bool;

    /// Check if this kind represents a type context (class, struct, enum, protocol, extension).
    fn is_type_context(&self) -> bool;
}

impl NodeKindExt for NodeKind {
    #[inline]
    fn is_function_type(&self) -> bool {
        matches!(
            self,
            NodeKind::FunctionType
                | NodeKind::NoEscapeFunctionType
                | NodeKind::CFunctionPointer
                | NodeKind::ThinFunctionType
                | NodeKind::ImplFunctionType
                | NodeKind::UncurriedFunctionType
        )
    }

    #[inline]
    fn is_type_context(&self) -> bool {
        matches!(
            self,
            NodeKind::Class
                | NodeKind::Structure
                | NodeKind::Enum
                | NodeKind::Protocol
                | NodeKind::Extension
        )
    }
}

/// Extension trait adding helper methods to [`Node`].
pub(crate) trait NodeExt<'ctx> {
    /// Find the module name from this node's children and descendants.
    ///
    /// Searches for a [`NodeKind::Module`] node in the following order:
    /// 1. Direct [`NodeKind::Module`] child
    /// 2. [`NodeKind::Module`] inside type context children ([`NodeKind::Class`]/[`NodeKind::Structure`]/[`NodeKind::Enum`]/[`NodeKind::Protocol`]/[`NodeKind::Extension`]/[`NodeKind::TypeAlias`])
    fn find_module(&self) -> Option<&'ctx str>;

    /// Find module by searching all descendants.
    ///
    /// Use this as a fallback when [`find_module`](NodeExt::find_module) doesn't find a result.
    fn find_module_in_descendants(&self) -> Option<&'ctx str>;

    /// Find the generic signature from this node's children.
    ///
    /// Searches for [`NodeKind::DependentGenericSignature`] in:
    /// 1. [`NodeKind::Type`] -> [`NodeKind::DependentGenericType`] -> [`NodeKind::DependentGenericSignature`]
    /// 2. [`NodeKind::Extension`] -> [`NodeKind::DependentGenericSignature`] (for constrained extensions)
    fn find_generic_signature(&self) -> Option<GenericSignature<'ctx>>;

    /// Find a function type from this node's children.
    ///
    /// Searches for function types ([`NodeKind::FunctionType`], [`NodeKind::NoEscapeFunctionType`], etc.) in:
    /// 1. [`NodeKind::Type`] -> [`NodeKind::FunctionType`] (direct)
    /// 2. [`NodeKind::Type`] -> [`NodeKind::DependentGenericType`] -> [`NodeKind::Type`] -> [`NodeKind::FunctionType`] (generic functions)
    fn find_function_type(&self) -> Option<FunctionType<'ctx>>;

    /// Extract argument labels from this node's [`NodeKind::LabelList`] child.
    ///
    /// Returns a vector where each element is `Some(label)` for labeled parameters
    /// and `None` for unlabeled parameters (using `_`).
    fn extract_labels(&self) -> Vec<Option<&'ctx str>>;

    /// Find the first [`NodeKind::Identifier`] child and return its text.
    fn find_identifier(&self) -> Option<&'ctx str>;

    /// Find identifier, also checking [`NodeKind::LocalDeclName`] and [`NodeKind::PrivateDeclName`] wrappers.
    ///
    /// This is useful for function names which may be wrapped in these nodes.
    fn find_identifier_extended(&self) -> Option<&'ctx str>;

    /// Find the containing type name from this node's children.
    ///
    /// Searches for [`NodeKind::Class`]/[`NodeKind::Structure`]/[`NodeKind::Enum`]/[`NodeKind::Protocol`] children and returns
    /// the [`NodeKind::Identifier`] inside. For [`NodeKind::Extension`], looks inside for the extended type.
    fn find_containing_type(&self) -> Option<&'ctx str>;

    /// Check if the containing type is a class (reference type).
    fn containing_type_is_class(&self) -> bool;

    /// Check if the containing type is a protocol.
    fn containing_type_is_protocol(&self) -> bool;

    /// Check if this node has a type context child ([`NodeKind::Class`]/[`NodeKind::Structure`]/[`NodeKind::Enum`]/[`NodeKind::Protocol`]/[`NodeKind::Extension`]).
    fn has_type_context(&self) -> bool;

    /// Find a [`NodeKind::Type`] child and extract its inner type as a [`TypeRef`].
    ///
    /// This handles the common pattern of [`NodeKind::Type`] -> inner type.
    fn extract_type_ref(&self) -> Option<TypeRef<'ctx>>;

    /// Find the first child with the given kind.
    fn child_of_kind(&self, kind: NodeKind) -> Option<Node<'ctx>>;

    /// If this node's kind matches, return its first child.
    ///
    /// Useful for unwrapping single-child wrapper nodes like [`NodeKind::Type`].
    /// Returns `None` if the kind doesn't match or the node has no children.
    fn unwrap_if_kind(&self, kind: NodeKind) -> Option<Node<'ctx>>;
}

impl<'ctx> NodeExt<'ctx> for Node<'ctx> {
    fn find_module(&self) -> Option<&'ctx str> {
        // Direct module child
        if let Some(module) = self.child_of_kind(NodeKind::Module) {
            return module.text();
        }
        // Module inside a type context
        for child in self.children() {
            match child.kind() {
                NodeKind::Class
                | NodeKind::Structure
                | NodeKind::Enum
                | NodeKind::Protocol
                | NodeKind::Extension
                | NodeKind::TypeAlias => {
                    for inner in child.descendants() {
                        if inner.kind() == NodeKind::Module {
                            return inner.text();
                        }
                    }
                }
                _ => {}
            }
        }
        None
    }

    fn find_module_in_descendants(&self) -> Option<&'ctx str> {
        for desc in self.descendants() {
            if desc.kind() == NodeKind::Module {
                return desc.text();
            }
        }
        None
    }

    fn find_generic_signature(&self) -> Option<GenericSignature<'ctx>> {
        // First, look for the symbol's own generic signature in Type -> DependentGenericType
        for child in self.children() {
            if let Some(inner) = child.unwrap_if_kind(NodeKind::Type)
                && inner.kind() == NodeKind::DependentGenericType
                && let Some(sig) = inner.child_of_kind(NodeKind::DependentGenericSignature)
            {
                return Some(GenericSignature::new(sig));
            }
        }
        // Fall back to constrained extension: Extension -> DependentGenericSignature
        // (only if the symbol itself doesn't have its own generic signature)
        if let Some(ext) = self.child_of_kind(NodeKind::Extension)
            && let Some(sig) = ext.child_of_kind(NodeKind::DependentGenericSignature)
        {
            return Some(GenericSignature::new(sig));
        }
        None
    }

    fn find_function_type(&self) -> Option<FunctionType<'ctx>> {
        for child in self.children() {
            if let Some(inner) = child.unwrap_if_kind(NodeKind::Type) {
                // Check for direct function type
                if inner.kind().is_function_type() {
                    return Some(FunctionType::new(inner));
                }
                // For generic functions, the function type is wrapped in DependentGenericType
                if inner.kind() == NodeKind::DependentGenericType {
                    for dep_child in inner.children() {
                        if let Some(func_type) = dep_child.unwrap_if_kind(NodeKind::Type)
                            && func_type.kind().is_function_type()
                        {
                            return Some(FunctionType::new(func_type));
                        }
                    }
                }
            }
        }
        None
    }

    fn extract_labels(&self) -> Vec<Option<&'ctx str>> {
        for child in self.children() {
            if child.kind() == NodeKind::LabelList {
                return child
                    .children()
                    .map(|label_node| {
                        if label_node.kind() == NodeKind::Identifier {
                            label_node.text()
                        } else if label_node.kind() == NodeKind::FirstElementMarker {
                            None // Represents `_`
                        } else {
                            label_node.text()
                        }
                    })
                    .collect();
            }
        }
        Vec::new()
    }

    fn find_identifier(&self) -> Option<&'ctx str> {
        for child in self.children() {
            if child.kind() == NodeKind::Identifier {
                return child.text();
            }
        }
        None
    }

    fn find_identifier_extended(&self) -> Option<&'ctx str> {
        // If this node itself is an Identifier, return its text
        if self.kind() == NodeKind::Identifier {
            return self.text();
        }
        for child in self.children() {
            if child.kind() == NodeKind::Identifier {
                return child.text();
            }
            // Check for LocalDeclName (for local functions)
            if child.kind() == NodeKind::LocalDeclName {
                for inner in child.children() {
                    if inner.kind() == NodeKind::Identifier {
                        return inner.text();
                    }
                }
            }
            // Check for PrivateDeclName
            if child.kind() == NodeKind::PrivateDeclName {
                for inner in child.children() {
                    if inner.kind() == NodeKind::Identifier {
                        return inner.text();
                    }
                }
            }
        }
        None
    }

    fn find_containing_type(&self) -> Option<&'ctx str> {
        for child in self.children() {
            match child.kind() {
                NodeKind::Class | NodeKind::Structure | NodeKind::Enum | NodeKind::Protocol => {
                    for inner in child.children() {
                        if inner.kind() == NodeKind::Identifier {
                            return inner.text();
                        }
                    }
                }
                NodeKind::Extension => {
                    // Extension wraps the extended type (e.g., Extension -> Protocol -> Identifier)
                    for inner in child.children() {
                        match inner.kind() {
                            NodeKind::Class
                            | NodeKind::Structure
                            | NodeKind::Enum
                            | NodeKind::Protocol => {
                                for id in inner.children() {
                                    if id.kind() == NodeKind::Identifier {
                                        return id.text();
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }
        None
    }

    fn containing_type_is_class(&self) -> bool {
        for child in self.children() {
            match child.kind() {
                NodeKind::Class => return true,
                NodeKind::Structure | NodeKind::Enum | NodeKind::Protocol => return false,
                NodeKind::Extension => {
                    for inner in child.children() {
                        match inner.kind() {
                            NodeKind::Class => return true,
                            NodeKind::Structure | NodeKind::Enum | NodeKind::Protocol => {
                                return false;
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }
        false
    }

    fn containing_type_is_protocol(&self) -> bool {
        for child in self.children() {
            match child.kind() {
                NodeKind::Protocol => return true,
                NodeKind::Class | NodeKind::Structure | NodeKind::Enum => return false,
                NodeKind::Extension => {
                    for inner in child.children() {
                        match inner.kind() {
                            NodeKind::Protocol => return true,
                            NodeKind::Class | NodeKind::Structure | NodeKind::Enum => {
                                return false;
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }
        false
    }

    fn has_type_context(&self) -> bool {
        self.children().any(|c| c.kind().is_type_context())
    }

    fn extract_type_ref(&self) -> Option<TypeRef<'ctx>> {
        let type_node = self.child_of_kind(NodeKind::Type)?;
        Some(TypeRef::new(type_node.child(0).unwrap_or(type_node)))
    }

    fn child_of_kind(&self, kind: NodeKind) -> Option<Node<'ctx>> {
        self.children().find(|c| c.kind() == kind)
    }

    fn unwrap_if_kind(&self, kind: NodeKind) -> Option<Node<'ctx>> {
        if self.kind() == kind {
            self.child(0)
        } else {
            None
        }
    }
}

/// Trait for types that may have a generic signature.
///
/// This provides a uniform interface for accessing generic constraints
/// on functions, constructors, closures, accessors, and enum cases.
pub trait HasGenericSignature<'ctx> {
    /// Get the generic signature if this symbol has generic constraints.
    fn generic_signature(&self) -> Option<GenericSignature<'ctx>>;

    /// Get the generic requirements (constraints) for this symbol.
    ///
    /// This is a convenience method that extracts just the requirements.
    fn generic_requirements(&self) -> Vec<GenericRequirement<'ctx>> {
        self.generic_signature()
            .map(|sig| sig.requirements())
            .unwrap_or_default()
    }

    /// Check if this symbol is generic.
    fn is_generic(&self) -> bool {
        self.generic_signature().is_some()
    }
}

/// Trait for types that have a function signature.
///
/// This provides a uniform interface for accessing function type information
/// on [`Function`](crate::Function)s, [`Constructor`](crate::Constructor)s, and [`Closure`](crate::Closure)s.
pub trait HasFunctionSignature<'ctx> {
    /// Get the function signature (type).
    fn signature(&self) -> Option<FunctionType<'ctx>>;


    /// Check if this function is async.
    fn is_async(&self) -> bool {
        self.signature().map(|s| s.is_async()).unwrap_or(false)
    }

    /// Check if this function throws.
    fn is_throwing(&self) -> bool {
        self.signature().map(|s| s.is_throwing()).unwrap_or(false)
    }
}

/// Trait for types that can be defined in an extension.
///
/// This provides a uniform interface for accessing extension context information
/// on functions, constructors, accessors, and closures.
pub trait HasExtensionContext<'ctx> {
    /// Get the raw node for this symbol.
    fn raw(&self) -> Node<'ctx>;

    /// Check if this symbol is defined in an extension.
    fn is_extension(&self) -> bool {
        self.raw()
            .children()
            .any(|c| c.kind() == NodeKind::Extension)
    }

    /// Get the module where the extension is defined, if this is an extension member.
    fn extension_module(&self) -> Option<&'ctx str> {
        for child in self.raw().children() {
            if child.kind() == NodeKind::Extension
                && let Some(module) = child.child_of_kind(NodeKind::Module)
            {
                return module.text();
            }
        }
        None
    }

    /// Get the generic signature from the extension context, if any.
    ///
    /// This is separate from [`HasGenericSignature::generic_signature`] which returns the
    /// symbol's own generic constraints. Extension constraints define
    /// when the extension applies (e.g., `extension Array where Element: Comparable`).
    fn extension_generic_signature(&self) -> Option<GenericSignature<'ctx>> {
        for child in self.raw().children() {
            if child.kind() == NodeKind::Extension
                && let Some(sig) = child.child_of_kind(NodeKind::DependentGenericSignature)
            {
                return Some(GenericSignature::new(sig));
            }
        }
        None
    }

    /// Get the generic requirements from the extension context, if any.
    fn extension_generic_requirements(&self) -> Vec<GenericRequirement<'ctx>> {
        self.extension_generic_signature()
            .map(|sig| sig.requirements())
            .unwrap_or_default()
    }
}

/// Trait for types that can provide their containing module.
pub trait HasModule<'ctx> {
    /// Get the module name where this symbol is defined.
    fn module(&self) -> Option<&'ctx str>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_function_type() {
        assert!(NodeKind::FunctionType.is_function_type());
        assert!(NodeKind::NoEscapeFunctionType.is_function_type());
        assert!(NodeKind::ThinFunctionType.is_function_type());
        assert!(!NodeKind::Module.is_function_type());
        assert!(!NodeKind::Class.is_function_type());
    }

    #[test]
    fn test_is_type_context() {
        assert!(NodeKind::Class.is_type_context());
        assert!(NodeKind::Structure.is_type_context());
        assert!(NodeKind::Enum.is_type_context());
        assert!(NodeKind::Protocol.is_type_context());
        assert!(NodeKind::Extension.is_type_context());
        assert!(!NodeKind::Module.is_type_context());
        assert!(!NodeKind::FunctionType.is_type_context());
    }
}
