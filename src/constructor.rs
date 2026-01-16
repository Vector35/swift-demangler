//! Constructor and destructor symbol representation.
//!
//! This module provides types for representing Swift initializers and deinitializers.

use crate::context::{SymbolContext, extract_context};
use crate::helpers::{
    HasExtensionContext, HasFunctionSignature, HasGenericSignature, HasModule, NodeExt,
};
use crate::raw::{Node, NodeKind};
use crate::types::{FunctionType, GenericSignature, TypeRef};

/// A Swift constructor (initializer) symbol.
#[derive(Clone, Copy)]
pub struct Constructor<'ctx> {
    raw: Node<'ctx>,
}

impl<'ctx> Constructor<'ctx> {
    /// Create a Constructor from a raw node.
    pub fn new(raw: Node<'ctx>) -> Self {
        Self { raw }
    }

    /// Get the underlying raw node.
    pub fn raw(&self) -> Node<'ctx> {
        self.raw
    }

    /// Get the context (location) where this constructor is defined.
    pub fn context(&self) -> SymbolContext<'ctx> {
        extract_context(self.raw)
    }

    /// Get the kind of constructor.
    pub fn kind(&self) -> ConstructorKind {
        match self.raw.kind() {
            NodeKind::Allocator => ConstructorKind::Allocating,
            NodeKind::Constructor => ConstructorKind::Regular,
            _ => ConstructorKind::Regular,
        }
    }

    /// Get the type containing this constructor.
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

    /// Get the argument labels for this constructor.
    pub fn labels(&self) -> Vec<Option<&'ctx str>> {
        self.raw.extract_labels()
    }

    /// Check if this is a failable initializer (returns optional).
    pub fn is_failable(&self) -> bool {
        if let Some(sig) = self.signature() {
            if let Some(ret) = sig.return_type() {
                // Check if return type is optional
                matches!(ret.kind(), crate::types::TypeKind::Optional(_))
            } else {
                false
            }
        } else {
            false
        }
    }
}

impl<'ctx> HasGenericSignature<'ctx> for Constructor<'ctx> {
    fn generic_signature(&self) -> Option<GenericSignature<'ctx>> {
        self.raw.find_generic_signature()
    }
}

impl<'ctx> HasFunctionSignature<'ctx> for Constructor<'ctx> {
    fn signature(&self) -> Option<FunctionType<'ctx>> {
        self.raw.find_function_type()
    }
}

impl<'ctx> HasExtensionContext<'ctx> for Constructor<'ctx> {
    fn raw(&self) -> Node<'ctx> {
        self.raw
    }
}

impl<'ctx> HasModule<'ctx> for Constructor<'ctx> {
    fn module(&self) -> Option<&'ctx str> {
        self.raw.find_module()
    }
}

impl std::fmt::Debug for Constructor<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = f.debug_struct("Constructor");
        s.field("kind", &self.kind())
            .field("containing_type", &self.containing_type())
            .field("module", &self.module())
            .field("labels", &self.labels())
            .field("is_async", &self.is_async())
            .field("is_throwing", &self.is_throwing())
            .field("is_failable", &self.is_failable())
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

impl std::fmt::Display for Constructor<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.raw)
    }
}

/// The kind of constructor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstructorKind {
    /// A regular constructor (init).
    Regular,
    /// An allocating constructor (allocates memory then initializes).
    Allocating,
}

/// A Swift destructor (deinitializer) symbol.
#[derive(Clone, Copy)]
pub struct Destructor<'ctx> {
    raw: Node<'ctx>,
}

impl<'ctx> Destructor<'ctx> {
    /// Create a Destructor from a raw node.
    pub fn new(raw: Node<'ctx>) -> Self {
        Self { raw }
    }

    /// Get the underlying raw node.
    pub fn raw(&self) -> Node<'ctx> {
        self.raw
    }

    /// Get the context (location) where this destructor is defined.
    pub fn context(&self) -> SymbolContext<'ctx> {
        extract_context(self.raw)
    }

    /// Get the kind of destructor.
    pub fn kind(&self) -> DestructorKind {
        match self.raw.kind() {
            NodeKind::Deallocator => DestructorKind::Deallocating,
            NodeKind::IsolatedDeallocator => DestructorKind::IsolatedDeallocating,
            NodeKind::Destructor => DestructorKind::Regular,
            _ => DestructorKind::Regular,
        }
    }

    /// Get the type containing this destructor.
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

    /// Get the type being destroyed.
    pub fn destroyed_type(&self) -> Option<TypeRef<'ctx>> {
        for child in self.raw.children() {
            match child.kind() {
                NodeKind::Class | NodeKind::Structure | NodeKind::Enum => {
                    return Some(TypeRef::new(child));
                }
                _ => {}
            }
        }
        None
    }
}

impl<'ctx> HasModule<'ctx> for Destructor<'ctx> {
    fn module(&self) -> Option<&'ctx str> {
        self.raw.find_module()
    }
}

impl std::fmt::Debug for Destructor<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Destructor")
            .field("kind", &self.kind())
            .field("containing_type", &self.containing_type())
            .field("module", &self.module())
            .finish()
    }
}

impl std::fmt::Display for Destructor<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.raw)
    }
}

/// The kind of destructor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DestructorKind {
    /// A regular destructor (deinit).
    Regular,
    /// A deallocating destructor (deinitializes then deallocates).
    Deallocating,
    /// An isolated deallocating destructor (for actors).
    IsolatedDeallocating,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constructor_kind() {
        assert_eq!(ConstructorKind::Regular, ConstructorKind::Regular);
        assert_ne!(ConstructorKind::Regular, ConstructorKind::Allocating);
    }

    #[test]
    fn test_destructor_kind() {
        assert_eq!(DestructorKind::Regular, DestructorKind::Regular);
        assert_ne!(DestructorKind::Regular, DestructorKind::Deallocating);
    }

    // Note: Full constructor/destructor tests require specific mangled symbols
    // which can be added from manglings.txt
}
