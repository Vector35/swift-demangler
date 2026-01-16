//! Type metadata symbol representation.
//!
//! Type metadata symbols represent runtime type information used by Swift
//! for dynamic dispatch, reflection, and generic instantiation.

use crate::helpers::NodeExt;
use crate::raw::{Node, NodeKind};
use crate::symbol::Symbol;
use crate::types::TypeRef;

/// A Swift type metadata symbol.
pub struct Metadata<'ctx> {
    raw: Node<'ctx>,
    inner: Option<Box<Symbol<'ctx>>>,
}

impl<'ctx> Metadata<'ctx> {
    /// Create a Metadata from a raw node.
    pub fn new(raw: Node<'ctx>) -> Self {
        Self { raw, inner: None }
    }

    /// Create a Metadata with an inner symbol (for sibling patterns like DefaultOverride).
    pub fn with_inner(raw: Node<'ctx>, inner: Symbol<'ctx>) -> Self {
        Self {
            raw,
            inner: Some(Box::new(inner)),
        }
    }

    /// Get the inner symbol, if any.
    pub fn inner(&self) -> Option<&Symbol<'ctx>> {
        self.inner.as_deref()
    }

    /// Get the underlying raw node.
    pub fn raw(&self) -> Node<'ctx> {
        self.raw
    }

    /// Get the kind of metadata.
    pub fn kind(&self) -> MetadataKind {
        match self.raw.kind() {
            NodeKind::TypeMetadata => MetadataKind::Type,
            NodeKind::FullTypeMetadata => MetadataKind::FullType,
            NodeKind::TypeMetadataAccessFunction => MetadataKind::AccessFunction,
            NodeKind::TypeMetadataCompletionFunction => MetadataKind::CompletionFunction,
            NodeKind::TypeMetadataInstantiationFunction => MetadataKind::InstantiationFunction,
            NodeKind::TypeMetadataInstantiationCache => MetadataKind::InstantiationCache,
            NodeKind::TypeMetadataLazyCache => MetadataKind::LazyCache,
            NodeKind::TypeMetadataSingletonInitializationCache => {
                MetadataKind::SingletonInitializationCache
            }
            NodeKind::TypeMetadataDemanglingCache => MetadataKind::DemanglingCache,
            NodeKind::GenericTypeMetadataPattern => MetadataKind::GenericPattern,
            NodeKind::MetadataInstantiationCache => MetadataKind::MetadataInstantiationCache,
            NodeKind::NoncanonicalSpecializedGenericTypeMetadata => {
                MetadataKind::NoncanonicalSpecializedGeneric
            }
            NodeKind::NoncanonicalSpecializedGenericTypeMetadataCache => {
                MetadataKind::NoncanonicalSpecializedGenericCache
            }
            NodeKind::CanonicalSpecializedGenericTypeMetadataAccessFunction => {
                MetadataKind::CanonicalSpecializedGenericAccessFunction
            }
            NodeKind::AssociatedTypeMetadataAccessor => MetadataKind::AssociatedTypeAccessor,
            NodeKind::DefaultAssociatedTypeMetadataAccessor => {
                MetadataKind::DefaultAssociatedTypeAccessor
            }
            NodeKind::ClassMetadataBaseOffset => MetadataKind::ClassBaseOffset,
            NodeKind::ObjCMetadataUpdateFunction => MetadataKind::ObjCUpdateFunction,
            NodeKind::FieldOffset => MetadataKind::FieldOffset,
            NodeKind::Metaclass => MetadataKind::Metaclass,
            NodeKind::IVarInitializer => MetadataKind::IVarInitializer,
            NodeKind::IVarDestroyer => MetadataKind::IVarDestroyer,
            NodeKind::HasSymbolQuery => MetadataKind::HasSymbolQuery,
            NodeKind::DefaultOverride => MetadataKind::DefaultOverride,
            NodeKind::PropertyWrapperBackingInitializer => {
                MetadataKind::PropertyWrapperBackingInitializer
            }
            NodeKind::MethodLookupFunction => MetadataKind::MethodLookupFunction,
            _ => MetadataKind::Other,
        }
    }

    /// Get the type this metadata is for.
    pub fn metadata_type(&self) -> Option<TypeRef<'ctx>> {
        // First try standard Type child
        if let Some(type_ref) = self.raw.extract_type_ref() {
            return Some(type_ref);
        }
        // Some metadata nodes have the type directly as a child
        for child in self.raw.children() {
            match child.kind() {
                NodeKind::Class
                | NodeKind::Structure
                | NodeKind::Enum
                | NodeKind::Protocol
                | NodeKind::BoundGenericClass
                | NodeKind::BoundGenericStructure
                | NodeKind::BoundGenericEnum => {
                    return Some(TypeRef::new(child));
                }
                _ => {}
            }
        }
        None
    }

    /// Check if this is a metadata accessor (function that returns metadata).
    pub fn is_accessor(&self) -> bool {
        matches!(
            self.kind(),
            MetadataKind::AccessFunction
                | MetadataKind::AssociatedTypeAccessor
                | MetadataKind::DefaultAssociatedTypeAccessor
                | MetadataKind::CanonicalSpecializedGenericAccessFunction
        )
    }

    /// Check if this is a cache for metadata.
    pub fn is_cache(&self) -> bool {
        matches!(
            self.kind(),
            MetadataKind::InstantiationCache
                | MetadataKind::LazyCache
                | MetadataKind::SingletonInitializationCache
                | MetadataKind::DemanglingCache
                | MetadataKind::MetadataInstantiationCache
                | MetadataKind::NoncanonicalSpecializedGenericCache
        )
    }
}

impl std::fmt::Debug for Metadata<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = f.debug_struct("Metadata");
        s.field("kind", &self.kind());
        if let Some(meta_type) = self.metadata_type() {
            s.field("metadata_type", &meta_type);
        }
        if let Some(inner) = &self.inner {
            s.field("inner", inner);
        }
        s.finish()
    }
}

impl std::fmt::Display for Metadata<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.raw)
    }
}

/// The kind of type metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetadataKind {
    /// Type metadata.
    Type,
    /// Full type metadata (includes value witness table pointer).
    FullType,
    /// Type metadata access function.
    AccessFunction,
    /// Type metadata completion function.
    CompletionFunction,
    /// Type metadata instantiation function (for generics).
    InstantiationFunction,
    /// Type metadata instantiation cache.
    InstantiationCache,
    /// Type metadata lazy cache.
    LazyCache,
    /// Type metadata singleton initialization cache.
    SingletonInitializationCache,
    /// Type metadata demangling cache.
    DemanglingCache,
    /// Generic type metadata pattern.
    GenericPattern,
    /// Metadata instantiation cache.
    MetadataInstantiationCache,
    /// Non-canonical specialized generic type metadata.
    NoncanonicalSpecializedGeneric,
    /// Non-canonical specialized generic type metadata cache.
    NoncanonicalSpecializedGenericCache,
    /// Canonical specialized generic type metadata access function.
    CanonicalSpecializedGenericAccessFunction,
    /// Associated type metadata accessor.
    AssociatedTypeAccessor,
    /// Default associated type metadata accessor.
    DefaultAssociatedTypeAccessor,
    /// Class metadata base offset.
    ClassBaseOffset,
    /// Objective-C metadata update function.
    ObjCUpdateFunction,
    /// Field offset.
    FieldOffset,
    /// Metaclass.
    Metaclass,
    /// Instance variable initializer.
    IVarInitializer,
    /// Instance variable destroyer.
    IVarDestroyer,
    /// Has symbol query (runtime availability check).
    HasSymbolQuery,
    /// Default override.
    DefaultOverride,
    /// Property wrapper backing initializer.
    PropertyWrapperBackingInitializer,
    /// Method lookup function (dynamic dispatch).
    MethodLookupFunction,
    /// Other metadata kind.
    Other,
}

impl MetadataKind {
    /// Get a human-readable name for this metadata kind.
    pub fn name(&self) -> &'static str {
        match self {
            MetadataKind::Type => "type metadata",
            MetadataKind::FullType => "full type metadata",
            MetadataKind::AccessFunction => "type metadata access function",
            MetadataKind::CompletionFunction => "type metadata completion function",
            MetadataKind::InstantiationFunction => "type metadata instantiation function",
            MetadataKind::InstantiationCache => "type metadata instantiation cache",
            MetadataKind::LazyCache => "type metadata lazy cache",
            MetadataKind::SingletonInitializationCache => {
                "type metadata singleton initialization cache"
            }
            MetadataKind::DemanglingCache => "type metadata demangling cache",
            MetadataKind::GenericPattern => "generic type metadata pattern",
            MetadataKind::MetadataInstantiationCache => "metadata instantiation cache",
            MetadataKind::NoncanonicalSpecializedGeneric => {
                "non-canonical specialized generic type metadata"
            }
            MetadataKind::NoncanonicalSpecializedGenericCache => {
                "non-canonical specialized generic type metadata cache"
            }
            MetadataKind::CanonicalSpecializedGenericAccessFunction => {
                "canonical specialized generic type metadata access function"
            }
            MetadataKind::AssociatedTypeAccessor => "associated type metadata accessor",
            MetadataKind::DefaultAssociatedTypeAccessor => {
                "default associated type metadata accessor"
            }
            MetadataKind::ClassBaseOffset => "class metadata base offset",
            MetadataKind::ObjCUpdateFunction => "Objective-C metadata update function",
            MetadataKind::FieldOffset => "field offset",
            MetadataKind::Metaclass => "metaclass",
            MetadataKind::IVarInitializer => "instance variable initializer",
            MetadataKind::IVarDestroyer => "instance variable destroyer",
            MetadataKind::HasSymbolQuery => "has symbol query",
            MetadataKind::DefaultOverride => "default override",
            MetadataKind::PropertyWrapperBackingInitializer => {
                "property wrapper backing initializer"
            }
            MetadataKind::MethodLookupFunction => "method lookup function",
            MetadataKind::Other => "metadata",
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::Symbol;
    use crate::raw::Context;

    #[test]
    fn test_type_metadata() {
        let ctx = Context::new();
        // type metadata for Swift.UInt16
        let symbol = Symbol::parse(&ctx, "$ss6UInt16VN").unwrap();
        assert!(symbol.is_metadata());
        if let Symbol::Metadata(meta) = symbol {
            assert_eq!(meta.kind(), super::MetadataKind::Type);
            let meta_type = meta.metadata_type().expect("should have metadata type");
            // Module info is in the type: Swift.UInt16
            assert!(meta_type.to_string().contains("Swift"));
        } else {
            panic!("Expected metadata");
        }
    }

    #[test]
    fn test_type_metadata_access_function() {
        let ctx = Context::new();
        // type metadata accessor for Swift.Int
        let symbol = Symbol::parse(&ctx, "$sSiMa").unwrap();
        assert!(symbol.is_metadata());
        if let Symbol::Metadata(meta) = symbol {
            assert_eq!(meta.kind(), super::MetadataKind::AccessFunction);
            assert!(meta.is_accessor());
            let meta_type = meta.metadata_type().expect("should have metadata type");
            // Module info is in the type: Swift.Int
            assert!(meta_type.to_string().contains("Swift"));
        } else {
            panic!("Expected metadata");
        }
    }
}
