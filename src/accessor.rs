//! Accessor symbol representation.
//!
//! This module provides types for representing Swift property accessors
//! (getters, setters, etc.).

use crate::context::{SymbolContext, extract_context};
use crate::helpers::{HasExtensionContext, HasGenericSignature, NodeExt, NodeKindExt};
use crate::raw::{Node, NodeKind};
use crate::types::{GenericSignature, TypeRef};

/// A Swift property accessor symbol.
#[derive(Clone, Copy)]
pub struct Accessor<'ctx> {
    raw: Node<'ctx>,
    is_static: bool,
}

impl<'ctx> Accessor<'ctx> {
    /// Create an Accessor from a raw node.
    pub fn new(raw: Node<'ctx>) -> Self {
        Self {
            raw,
            is_static: false,
        }
    }

    /// Create an Accessor from a raw node, marking it as static.
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

    /// Get the context (location) where this accessor is defined.
    pub fn context(&self) -> SymbolContext<'ctx> {
        extract_context(self.raw)
    }

    /// Get the kind of accessor.
    pub fn kind(&self) -> AccessorKind {
        match self.raw.kind() {
            NodeKind::Getter => AccessorKind::Getter,
            NodeKind::Setter => AccessorKind::Setter,
            NodeKind::ModifyAccessor | NodeKind::Modify2Accessor => AccessorKind::Modify,
            NodeKind::ReadAccessor | NodeKind::Read2Accessor => AccessorKind::Read,
            NodeKind::WillSet => AccessorKind::WillSet,
            NodeKind::DidSet => AccessorKind::DidSet,
            NodeKind::GlobalGetter => AccessorKind::GlobalGetter,
            NodeKind::MaterializeForSet => AccessorKind::MaterializeForSet,
            NodeKind::InitAccessor => AccessorKind::Init,
            NodeKind::UnsafeAddressor => AccessorKind::UnsafeAddressor,
            NodeKind::UnsafeMutableAddressor => AccessorKind::UnsafeMutableAddressor,
            NodeKind::OwningAddressor => AccessorKind::OwningAddressor,
            NodeKind::OwningMutableAddressor => AccessorKind::OwningMutableAddressor,
            NodeKind::NativeOwningAddressor => AccessorKind::NativeOwningAddressor,
            NodeKind::NativeOwningMutableAddressor => AccessorKind::NativeOwningMutableAddressor,
            NodeKind::NativePinningAddressor => AccessorKind::NativePinningAddressor,
            NodeKind::NativePinningMutableAddressor => AccessorKind::NativePinningMutableAddressor,
            // Bare subscript node (old-style mangling) - treat as subscript getter
            NodeKind::Subscript => AccessorKind::Subscript,
            _ => AccessorKind::Other,
        }
    }

    /// Get the property name.
    pub fn property_name(&self) -> Option<&'ctx str> {
        // The property name is usually found in a Variable child
        for child in self.raw.children() {
            if child.kind() == NodeKind::Variable {
                for inner in child.children() {
                    if inner.kind() == NodeKind::Identifier {
                        return inner.text();
                    }
                }
            }
            // Or directly in an Identifier child
            if child.kind() == NodeKind::Identifier {
                return child.text();
            }
            // Check for Subscript
            if child.kind() == NodeKind::Subscript {
                return Some("subscript");
            }
        }
        None
    }

    /// Get the property type.
    pub fn property_type(&self) -> Option<TypeRef<'ctx>> {
        // Look for Type child in Variable
        for child in self.raw.children() {
            if child.kind() == NodeKind::Variable
                && let Some(type_ref) = child.extract_type_ref()
            {
                return Some(type_ref);
            }
        }
        // Direct Type child
        self.raw.extract_type_ref()
    }

    /// Get the module containing this accessor.
    pub fn module(&self) -> Option<&'ctx str> {
        // Direct module child
        for child in self.raw.children() {
            if child.kind() == NodeKind::Module {
                return child.text();
            }
        }
        // Check Variable -> containing type -> Module
        // (not Type, which is the property type, not the containing type)
        for child in self.raw.children() {
            if child.kind() == NodeKind::Variable {
                // Variable's children: [ContainingType, Identifier, Type]
                // We want the module from ContainingType, not from Type
                for inner in child.children() {
                    if inner.kind() == NodeKind::Module {
                        return inner.text();
                    }
                    if inner.kind().is_type_context() {
                        // Search only within the containing type
                        if let Some(module) = inner.find_module_in_descendants() {
                            return Some(module);
                        }
                    }
                }
            }
        }
        // Module inside a direct type context (for non-Variable cases)
        for child in self.raw.children() {
            if child.kind().is_type_context()
                && let Some(module) = child.find_module_in_descendants()
            {
                return Some(module);
            }
        }
        None
    }

    /// Get the containing type if this is a computed property or method accessor.
    ///
    /// For nested types, returns the full dot-separated path (e.g., "ArchiveEncryptionContext.Profile").
    pub fn containing_type(&self) -> Option<String> {
        self.find_containing_type_node()
            .map(|node| self.build_type_path(node))
    }

    /// Check if the containing type is a class (reference type).
    pub fn containing_type_is_class(&self) -> bool {
        self.find_containing_type_node()
            .map(|node| node.kind() == NodeKind::Class)
            .unwrap_or(false)
    }

    fn find_containing_type_node(&self) -> Option<Node<'ctx>> {
        for child in self.raw.children() {
            if child.kind() == NodeKind::Variable {
                for inner in child.children() {
                    if inner.kind().is_type_context() {
                        return Some(inner);
                    }
                }
            }
            if child.kind().is_type_context() {
                return Some(child);
            }
        }
        None
    }

    fn extract_type_name(&self, node: Node<'ctx>) -> Option<&'ctx str> {
        node.find_identifier()
    }

    fn build_type_path(&self, node: Node<'ctx>) -> String {
        let mut components = Vec::new();
        self.collect_type_path_components(node, &mut components);
        components.join(".")
    }

    fn collect_type_path_components(&self, node: Node<'ctx>, components: &mut Vec<&'ctx str>) {
        // Handle Extension by looking inside for the extended type
        if node.kind() == NodeKind::Extension {
            for child in node.children() {
                if child.kind().is_type_context() || child.kind() == NodeKind::TypeAlias {
                    self.collect_type_path_components(child, components);
                    return;
                }
            }
            return;
        }

        // First, recurse into nested type contexts (e.g., Class inside Structure)
        for child in node.children() {
            if child.kind().is_type_context() || child.kind() == NodeKind::TypeAlias {
                self.collect_type_path_components(child, components);
            }
        }
        // Then add this node's identifier
        if let Some(name) = self.extract_type_name(node) {
            components.push(name);
        }
    }

    /// Check if this is a subscript accessor.
    pub fn is_subscript(&self) -> bool {
        self.raw.kind() == NodeKind::Subscript
            || self.raw.children().any(|c| c.kind() == NodeKind::Subscript)
    }

    fn find_extension_node(&self) -> Option<Node<'ctx>> {
        for child in self.raw.children() {
            if child.kind() == NodeKind::Variable {
                for inner in child.children() {
                    if inner.kind() == NodeKind::Extension {
                        return Some(inner);
                    }
                }
            }
            if child.kind() == NodeKind::Extension {
                return Some(child);
            }
        }
        None
    }

    /// Check if this is a static property accessor.
    pub fn is_static(&self) -> bool {
        self.is_static
    }

    /// Check if this accessor mutates the property.
    pub fn is_mutating(&self) -> bool {
        matches!(
            self.kind(),
            AccessorKind::Setter
                | AccessorKind::Modify
                | AccessorKind::WillSet
                | AccessorKind::DidSet
                | AccessorKind::MaterializeForSet
                | AccessorKind::Init
                | AccessorKind::UnsafeMutableAddressor
                | AccessorKind::OwningMutableAddressor
                | AccessorKind::NativeOwningMutableAddressor
                | AccessorKind::NativePinningMutableAddressor
        )
    }
}

impl<'ctx> HasGenericSignature<'ctx> for Accessor<'ctx> {
    fn generic_signature(&self) -> Option<GenericSignature<'ctx>> {
        // Look for Type child that contains DependentGenericType
        for child in self.raw.children() {
            if let Some(inner) = child.unwrap_if_kind(NodeKind::Type)
                && inner.kind() == NodeKind::DependentGenericType
                && let Some(sig) = inner.child_of_kind(NodeKind::DependentGenericSignature)
            {
                return Some(GenericSignature::new(sig));
            }
            // Check inside Subscript or Variable
            if matches!(child.kind(), NodeKind::Subscript | NodeKind::Variable) {
                for inner in child.children() {
                    if let Some(type_inner) = inner.unwrap_if_kind(NodeKind::Type)
                        && type_inner.kind() == NodeKind::DependentGenericType
                        && let Some(sig) =
                            type_inner.child_of_kind(NodeKind::DependentGenericSignature)
                    {
                        return Some(GenericSignature::new(sig));
                    }
                }
            }
        }
        None
    }
}

impl<'ctx> HasExtensionContext<'ctx> for Accessor<'ctx> {
    fn raw(&self) -> Node<'ctx> {
        self.raw
    }

    // Override because Accessor needs to look inside Variable children too
    fn is_extension(&self) -> bool {
        self.find_extension_node().is_some()
    }

    fn extension_module(&self) -> Option<&'ctx str> {
        self.find_extension_node()
            .and_then(|ext| ext.child_of_kind(NodeKind::Module))
            .and_then(|m| m.text())
    }

    fn extension_generic_signature(&self) -> Option<GenericSignature<'ctx>> {
        self.find_extension_node()
            .and_then(|ext| ext.child_of_kind(NodeKind::DependentGenericSignature))
            .map(GenericSignature::new)
    }
}

impl std::fmt::Debug for Accessor<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = f.debug_struct("Accessor");
        s.field("kind", &self.kind())
            .field("property_name", &self.property_name())
            .field("property_type", &self.property_type())
            .field("module", &self.module())
            .field("containing_type", &self.containing_type())
            .field("is_static", &self.is_static())
            .field("is_mutating", &self.is_mutating())
            .field("is_subscript", &self.is_subscript())
            .field("is_extension", &self.is_extension())
            .field("is_generic", &self.is_generic());
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

impl std::fmt::Display for Accessor<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.raw)
    }
}

/// The kind of accessor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessorKind {
    /// A getter.
    Getter,
    /// A setter.
    Setter,
    /// A modify accessor (for inout access).
    Modify,
    /// A read accessor (for borrowing).
    Read,
    /// A willSet observer.
    WillSet,
    /// A didSet observer.
    DidSet,
    /// A global getter.
    GlobalGetter,
    /// materializeForSet accessor (older Swift).
    MaterializeForSet,
    /// An init accessor.
    Init,
    /// An unsafe addressor.
    UnsafeAddressor,
    /// An unsafe mutable addressor.
    UnsafeMutableAddressor,
    /// An owning addressor.
    OwningAddressor,
    /// An owning mutable addressor.
    OwningMutableAddressor,
    /// A native owning addressor.
    NativeOwningAddressor,
    /// A native owning mutable addressor.
    NativeOwningMutableAddressor,
    /// A native pinning addressor.
    NativePinningAddressor,
    /// A native pinning mutable addressor.
    NativePinningMutableAddressor,
    /// A subscript (when no explicit getter/setter node).
    Subscript,
    /// Other accessor kind.
    Other,
}

impl AccessorKind {
    /// Get a human-readable name for this accessor kind.
    pub fn name(&self) -> &'static str {
        match self {
            AccessorKind::Getter => "getter",
            AccessorKind::Setter => "setter",
            AccessorKind::Modify => "modify",
            AccessorKind::Read => "read",
            AccessorKind::WillSet => "willSet",
            AccessorKind::DidSet => "didSet",
            AccessorKind::GlobalGetter => "globalGetter",
            AccessorKind::MaterializeForSet => "materializeForSet",
            AccessorKind::Init => "init",
            AccessorKind::UnsafeAddressor => "unsafeAddressor",
            AccessorKind::UnsafeMutableAddressor => "unsafeMutableAddressor",
            AccessorKind::OwningAddressor => "owningAddressor",
            AccessorKind::OwningMutableAddressor => "owningMutableAddressor",
            AccessorKind::NativeOwningAddressor => "nativeOwningAddressor",
            AccessorKind::NativeOwningMutableAddressor => "nativeOwningMutableAddressor",
            AccessorKind::NativePinningAddressor => "nativePinningAddressor",
            AccessorKind::NativePinningMutableAddressor => "nativePinningMutableAddressor",
            AccessorKind::Subscript => "subscript",
            AccessorKind::Other => "other",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::raw::Context;
    use crate::symbol::Symbol;

    #[test]
    fn test_getter() {
        let ctx = Context::new();
        // foo.bar.getter : Swift.Int
        let symbol = Symbol::parse(&ctx, "_TF3foog3barSi").unwrap();
        assert!(symbol.is_accessor());
        if let Symbol::Accessor(acc) = symbol {
            assert_eq!(acc.kind(), AccessorKind::Getter);
            assert_eq!(acc.property_name(), Some("bar"));
            assert_eq!(acc.module(), Some("foo"));
            assert!(!acc.is_mutating());
        } else {
            panic!("Expected accessor");
        }
    }

    #[test]
    fn test_setter() {
        let ctx = Context::new();
        // foo.bar.setter : Swift.Int
        let symbol = Symbol::parse(&ctx, "_TF3foos3barSi").unwrap();
        assert!(symbol.is_accessor());
        if let Symbol::Accessor(acc) = symbol {
            assert_eq!(acc.kind(), AccessorKind::Setter);
            assert_eq!(acc.property_name(), Some("bar"));
            assert!(acc.is_mutating());
        } else {
            panic!("Expected accessor");
        }
    }

    #[test]
    fn test_unsafe_addressor() {
        let ctx = Context::new();
        // foo.bar.unsafeAddressor : Swift.Int
        let symbol = Symbol::parse(&ctx, "_TF3foolu3barSi").unwrap();
        assert!(symbol.is_accessor());
        if let Symbol::Accessor(acc) = symbol {
            assert_eq!(acc.kind(), AccessorKind::UnsafeAddressor);
            assert!(!acc.is_mutating());
        } else {
            panic!("Expected accessor");
        }
    }

    #[test]
    fn test_unsafe_mutable_addressor() {
        let ctx = Context::new();
        // foo.bar.unsafeMutableAddressor : Swift.Int
        let symbol = Symbol::parse(&ctx, "_TF3fooau3barSi").unwrap();
        assert!(symbol.is_accessor());
        if let Symbol::Accessor(acc) = symbol {
            assert_eq!(acc.kind(), AccessorKind::UnsafeMutableAddressor);
            assert!(acc.is_mutating());
        } else {
            panic!("Expected accessor");
        }
    }

    #[test]
    fn test_accessor_kind_name() {
        assert_eq!(AccessorKind::Getter.name(), "getter");
        assert_eq!(AccessorKind::Setter.name(), "setter");
        assert_eq!(AccessorKind::Modify.name(), "modify");
    }
}
