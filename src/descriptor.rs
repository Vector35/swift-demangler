//! Metadata descriptor symbol representation.
//!
//! Descriptors are metadata structures that describe various Swift runtime
//! information like protocol conformances, type metadata, etc.

use crate::function::Function;
use crate::helpers::{HasGenericSignature, NodeExt, NodeKindExt};
use crate::raw::{Node, NodeKind};
use crate::types::{GenericRequirement, GenericSignature, TypeRef};
use crate::witness_table::ProtocolConformance;

/// A Swift metadata descriptor symbol.
#[derive(Clone, Copy)]
pub struct Descriptor<'ctx> {
    raw: Node<'ctx>,
}

impl<'ctx> Descriptor<'ctx> {
    /// Create a Descriptor from a raw node.
    pub fn new(raw: Node<'ctx>) -> Self {
        Self { raw }
    }

    /// Get the underlying raw node.
    pub fn raw(&self) -> Node<'ctx> {
        self.raw
    }

    /// Get the kind of descriptor.
    pub fn kind(&self) -> DescriptorKind {
        match self.raw.kind() {
            NodeKind::ProtocolConformanceDescriptor => DescriptorKind::ProtocolConformance,
            NodeKind::ProtocolConformanceDescriptorRecord => {
                DescriptorKind::ProtocolConformanceRecord
            }
            NodeKind::OpaqueTypeDescriptor => DescriptorKind::OpaqueType,
            NodeKind::OpaqueTypeDescriptorRecord => DescriptorKind::OpaqueTypeRecord,
            NodeKind::OpaqueTypeDescriptorAccessor => DescriptorKind::OpaqueTypeAccessor,
            NodeKind::OpaqueTypeDescriptorAccessorImpl => DescriptorKind::OpaqueTypeAccessorImpl,
            NodeKind::OpaqueTypeDescriptorAccessorKey => DescriptorKind::OpaqueTypeAccessorKey,
            NodeKind::OpaqueTypeDescriptorAccessorVar => DescriptorKind::OpaqueTypeAccessorVar,
            NodeKind::NominalTypeDescriptor => DescriptorKind::NominalType,
            NodeKind::NominalTypeDescriptorRecord => DescriptorKind::NominalTypeRecord,
            NodeKind::PropertyDescriptor => DescriptorKind::Property,
            NodeKind::ProtocolDescriptor => DescriptorKind::Protocol,
            NodeKind::ProtocolDescriptorRecord => DescriptorKind::ProtocolRecord,
            NodeKind::ProtocolRequirementsBaseDescriptor => {
                DescriptorKind::ProtocolRequirementsBase
            }
            NodeKind::MethodDescriptor => DescriptorKind::Method,
            NodeKind::AssociatedTypeDescriptor => DescriptorKind::AssociatedType,
            NodeKind::AssociatedConformanceDescriptor => DescriptorKind::AssociatedConformance,
            NodeKind::DefaultAssociatedConformanceAccessor => {
                DescriptorKind::DefaultAssociatedConformanceAccessor
            }
            NodeKind::BaseConformanceDescriptor => DescriptorKind::BaseConformance,
            NodeKind::ExtensionDescriptor => DescriptorKind::Extension,
            NodeKind::AnonymousDescriptor => DescriptorKind::Anonymous,
            NodeKind::ModuleDescriptor => DescriptorKind::Module,
            NodeKind::ReflectionMetadataAssocTypeDescriptor => {
                DescriptorKind::ReflectionMetadataAssocType
            }
            NodeKind::AccessibleFunctionRecord => DescriptorKind::AccessibleFunctionRecord,
            _ => DescriptorKind::Other,
        }
    }

    /// Get the protocol conformance information if this is a ProtocolConformanceDescriptor.
    pub fn conformance(&self) -> Option<ProtocolConformance<'ctx>> {
        self.raw
            .child_of_kind(NodeKind::ProtocolConformance)
            .map(ProtocolConformance::new)
    }

    /// Get the conforming type if this is a ProtocolConformanceDescriptor.
    pub fn conforming_type(&self) -> Option<TypeRef<'ctx>> {
        self.conformance().and_then(|c| c.conforming_type())
    }

    /// Get the protocol if this is a ProtocolConformanceDescriptor.
    pub fn protocol(&self) -> Option<TypeRef<'ctx>> {
        self.conformance().and_then(|c| c.protocol())
    }

    /// Get the module where this descriptor is defined.
    pub fn module(&self) -> Option<&'ctx str> {
        // First try the conformance module
        if let Some(conformance) = self.conformance()
            && let Some(module) = conformance.module()
        {
            return Some(module);
        }
        // Otherwise search descendants
        self.raw.find_module_in_descendants()
    }

    /// Get the described type for type descriptors.
    pub fn described_type(&self) -> Option<TypeRef<'ctx>> {
        // First try standard Type child
        if let Some(type_ref) = self.raw.extract_type_ref() {
            return Some(type_ref);
        }
        // Direct type node children (for NominalTypeDescriptor, ClassDescriptor, etc.)
        for child in self.raw.children() {
            if child.kind().is_type_context() || child.kind() == NodeKind::TypeAlias {
                return Some(TypeRef::new(child));
            }
        }
        None
    }

    /// Get the described function for method descriptors.
    ///
    /// For MethodDescriptor symbols, this returns the inner function that
    /// the descriptor describes, providing access to the full method signature,
    /// parameters, generic constraints, etc.
    pub fn described_function(&self) -> Option<Function<'ctx>> {
        if self.kind() != DescriptorKind::Method {
            return None;
        }
        for child in self.raw.children() {
            if child.kind() == NodeKind::Function {
                return Some(Function::new(child));
            }
        }
        None
    }

    /// Get the function name if this is a method descriptor.
    pub fn function_name(&self) -> Option<&'ctx str> {
        self.described_function().and_then(|f| f.name())
    }

    /// Get the containing type name if this is a method descriptor.
    pub fn containing_type(&self) -> Option<&'ctx str> {
        self.described_function().and_then(|f| f.containing_type())
    }

    /// Get the generic signature if this descriptor has generic constraints.
    pub fn generic_signature(&self) -> Option<GenericSignature<'ctx>> {
        self.described_function()
            .and_then(|f| f.generic_signature())
    }

    /// Get the generic requirements (constraints) for this descriptor.
    pub fn generic_requirements(&self) -> Vec<GenericRequirement<'ctx>> {
        self.generic_signature()
            .map(|sig| sig.requirements())
            .unwrap_or_default()
    }

    /// Check if this descriptor is generic.
    pub fn is_generic(&self) -> bool {
        self.generic_signature().is_some()
    }
}

impl std::fmt::Debug for Descriptor<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut debug = f.debug_struct("Descriptor");
        debug.field("kind", &self.kind());

        if let Some(conformance) = self.conformance() {
            debug.field("conforming_type", &conformance.conforming_type());
            debug.field("protocol", &conformance.protocol());
            debug.field("conformance_module", &conformance.module());
        } else if let Some(func) = self.described_function() {
            debug.field("function_name", &func.name());
            debug.field("containing_type", &func.containing_type());
            debug.field("is_generic", &self.is_generic());
        } else if let Some(described) = self.described_type() {
            debug.field("described_type", &described);
        }

        debug.field("module", &self.module());
        debug.finish()
    }
}

impl std::fmt::Display for Descriptor<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.raw)
    }
}

/// The kind of metadata descriptor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DescriptorKind {
    /// Protocol conformance descriptor.
    ProtocolConformance,
    /// Protocol conformance descriptor record.
    ProtocolConformanceRecord,
    /// Opaque type descriptor.
    OpaqueType,
    /// Opaque type descriptor record.
    OpaqueTypeRecord,
    /// Opaque type descriptor accessor.
    OpaqueTypeAccessor,
    /// Opaque type descriptor accessor implementation.
    OpaqueTypeAccessorImpl,
    /// Opaque type descriptor accessor key.
    OpaqueTypeAccessorKey,
    /// Opaque type descriptor accessor variable.
    OpaqueTypeAccessorVar,
    /// Nominal type descriptor.
    NominalType,
    /// Nominal type descriptor record.
    NominalTypeRecord,
    /// Property descriptor.
    Property,
    /// Protocol descriptor.
    Protocol,
    /// Protocol descriptor record.
    ProtocolRecord,
    /// Protocol requirements base descriptor.
    ProtocolRequirementsBase,
    /// Method descriptor.
    Method,
    /// Associated type descriptor.
    AssociatedType,
    /// Associated conformance descriptor.
    AssociatedConformance,
    /// Default associated conformance accessor.
    DefaultAssociatedConformanceAccessor,
    /// Base conformance descriptor.
    BaseConformance,
    /// Extension descriptor.
    Extension,
    /// Anonymous descriptor.
    Anonymous,
    /// Module descriptor.
    Module,
    /// Reflection metadata associated type descriptor.
    ReflectionMetadataAssocType,
    /// Accessible function record.
    AccessibleFunctionRecord,
    /// Other descriptor kind.
    Other,
}

impl DescriptorKind {
    /// Get a human-readable name for this descriptor kind.
    pub fn name(&self) -> &'static str {
        match self {
            DescriptorKind::ProtocolConformance => "protocol conformance descriptor",
            DescriptorKind::ProtocolConformanceRecord => "protocol conformance descriptor record",
            DescriptorKind::OpaqueType => "opaque type descriptor",
            DescriptorKind::OpaqueTypeRecord => "opaque type descriptor record",
            DescriptorKind::OpaqueTypeAccessor => "opaque type descriptor accessor",
            DescriptorKind::OpaqueTypeAccessorImpl => "opaque type descriptor accessor impl",
            DescriptorKind::OpaqueTypeAccessorKey => "opaque type descriptor accessor key",
            DescriptorKind::OpaqueTypeAccessorVar => "opaque type descriptor accessor var",
            DescriptorKind::NominalType => "nominal type descriptor",
            DescriptorKind::NominalTypeRecord => "nominal type descriptor record",
            DescriptorKind::Property => "property descriptor",
            DescriptorKind::Protocol => "protocol descriptor",
            DescriptorKind::ProtocolRecord => "protocol descriptor record",
            DescriptorKind::ProtocolRequirementsBase => "protocol requirements base descriptor",
            DescriptorKind::Method => "method descriptor",
            DescriptorKind::AssociatedType => "associated type descriptor",
            DescriptorKind::AssociatedConformance => "associated conformance descriptor",
            DescriptorKind::DefaultAssociatedConformanceAccessor => {
                "default associated conformance accessor"
            }
            DescriptorKind::BaseConformance => "base conformance descriptor",
            DescriptorKind::Extension => "extension descriptor",
            DescriptorKind::Anonymous => "anonymous descriptor",
            DescriptorKind::Module => "module descriptor",
            DescriptorKind::ReflectionMetadataAssocType => {
                "reflection metadata associated type descriptor"
            }
            DescriptorKind::AccessibleFunctionRecord => "accessible function record",
            DescriptorKind::Other => "descriptor",
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::Symbol;
    use crate::helpers::HasGenericSignature;
    use crate::raw::Context;

    #[test]
    fn test_protocol_conformance_descriptor() {
        let ctx = Context::new();
        // protocol conformance descriptor for (extension in Combine):Swift.Result<A, B>.Publisher : Combine.Publisher
        let symbol = Symbol::parse(&ctx, "$ss6ResultO7CombineE9PublisherVyxq__GAcdCMc").unwrap();
        assert!(symbol.is_descriptor());
        if let Symbol::Descriptor(desc) = symbol {
            assert_eq!(desc.kind(), super::DescriptorKind::ProtocolConformance);
            assert!(desc.conformance().is_some());
            assert!(desc.conforming_type().is_some());
            assert!(desc.protocol().is_some());
            assert_eq!(desc.module(), Some("Combine"));
        } else {
            panic!("Expected descriptor");
        }
    }

    #[test]
    fn test_method_descriptor() {
        let ctx = Context::new();
        // method descriptor for Swift.Encoder.container<A where A1: Swift.CodingKey>(keyedBy: A1.Type) -> Swift.KeyedEncodingContainer<A1>
        let symbol =
            Symbol::parse(&ctx, "$ss7EncoderP9container7keyedBys22KeyedEncodingContainerVyqd__Gqd__m_ts9CodingKeyRd__lFTq").unwrap();
        assert!(symbol.is_descriptor());
        if let Symbol::Descriptor(desc) = symbol {
            assert_eq!(desc.kind(), super::DescriptorKind::Method);

            // Check described function
            let func = desc.described_function();
            assert!(func.is_some());
            let func = func.unwrap();
            assert_eq!(func.name(), Some("container"));
            assert_eq!(func.containing_type(), Some("Encoder"));
            assert!(func.is_generic());

            // Check generic requirements
            let requirements = desc.generic_requirements();
            assert!(!requirements.is_empty());

            assert_eq!(desc.module(), Some("Swift"));
        } else {
            panic!("Expected descriptor");
        }
    }
}
