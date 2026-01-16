//! Witness table symbol representation.
//!
//! Witness tables are data structures that enable protocol-oriented programming
//! in Swift by storing the implementations of protocol requirements for a
//! conforming type.

use crate::helpers::NodeExt;
use crate::raw::{Node, NodeKind};
use crate::types::TypeRef;

/// A Swift witness table symbol.
#[derive(Clone, Copy)]
pub struct WitnessTable<'ctx> {
    raw: Node<'ctx>,
}

impl<'ctx> WitnessTable<'ctx> {
    /// Create a WitnessTable from a raw node.
    pub fn new(raw: Node<'ctx>) -> Self {
        Self { raw }
    }

    /// Get the underlying raw node.
    pub fn raw(&self) -> Node<'ctx> {
        self.raw
    }

    /// Get the kind of witness table.
    pub fn kind(&self) -> WitnessTableKind {
        match self.raw.kind() {
            NodeKind::ProtocolWitnessTable => WitnessTableKind::Protocol,
            NodeKind::ProtocolWitnessTableAccessor => WitnessTableKind::ProtocolAccessor,
            NodeKind::ProtocolWitnessTablePattern => WitnessTableKind::ProtocolPattern,
            NodeKind::GenericProtocolWitnessTable => WitnessTableKind::GenericProtocol,
            NodeKind::GenericProtocolWitnessTableInstantiationFunction => {
                WitnessTableKind::GenericProtocolInstantiation
            }
            NodeKind::ResilientProtocolWitnessTable => WitnessTableKind::ResilientProtocol,
            NodeKind::LazyProtocolWitnessTableAccessor => WitnessTableKind::LazyProtocolAccessor,
            NodeKind::LazyProtocolWitnessTableCacheVariable => {
                WitnessTableKind::LazyProtocolCacheVariable
            }
            NodeKind::ProtocolSelfConformanceWitnessTable => WitnessTableKind::SelfConformance,
            NodeKind::ValueWitnessTable => WitnessTableKind::ValueTable,
            NodeKind::ValueWitness => WitnessTableKind::ValueWitness,
            NodeKind::AssociatedTypeWitnessTableAccessor => {
                WitnessTableKind::AssociatedTypeAccessor
            }
            NodeKind::BaseWitnessTableAccessor => WitnessTableKind::BaseAccessor,
            NodeKind::ConcreteProtocolConformance => WitnessTableKind::ConcreteConformance,
            _ => WitnessTableKind::Other,
        }
    }

    /// Get the value witness kind if this is a ValueWitness.
    pub fn value_witness_kind(&self) -> Option<ValueWitnessKind> {
        if self.raw.kind() != NodeKind::ValueWitness {
            return None;
        }
        // The index child indicates which value witness
        for child in self.raw.children() {
            if child.kind() == NodeKind::Index {
                return child.index().and_then(ValueWitnessKind::from_index);
            }
        }
        None
    }

    /// Get the protocol conformance information if available.
    pub fn conformance(&self) -> Option<ProtocolConformance<'ctx>> {
        self.raw
            .child_of_kind(NodeKind::ProtocolConformance)
            .map(ProtocolConformance::new)
    }

    /// Get the conforming type.
    pub fn conforming_type(&self) -> Option<TypeRef<'ctx>> {
        if let Some(conformance) = self.conformance() {
            return conformance.conforming_type();
        }
        // For ValueWitnessTable, the type is a direct child
        self.raw.extract_type_ref()
    }

    /// Get the protocol being conformed to.
    pub fn protocol(&self) -> Option<TypeRef<'ctx>> {
        self.conformance().and_then(|c| c.protocol())
    }

    /// Get the module where the conformance is declared.
    pub fn conformance_module(&self) -> Option<&'ctx str> {
        self.conformance().and_then(|c| c.module())
    }

    /// Get the module of the conforming type.
    pub fn module(&self) -> Option<&'ctx str> {
        // First try the conformance module
        if let Some(module) = self.conformance_module() {
            return Some(module);
        }
        // Otherwise search descendants
        self.raw.find_module_in_descendants()
    }

    /// Get the associated type path for AssociatedTypeWitnessTableAccessor.
    ///
    /// Returns the path as a list of names (e.g., ["A", "ZZ"] for A.ZZ).
    pub fn associated_type_path(&self) -> Vec<&'ctx str> {
        if self.raw.kind() != NodeKind::AssociatedTypeWitnessTableAccessor {
            return Vec::new();
        }
        for child in self.raw.children() {
            if child.kind() == NodeKind::AssocTypePath {
                return child
                    .children()
                    .filter_map(|c| {
                        if c.kind() == NodeKind::DependentAssociatedTypeRef {
                            // DependentAssociatedTypeRef -> Identifier
                            c.child(0).and_then(|id| id.text())
                        } else {
                            None
                        }
                    })
                    .collect();
            }
        }
        Vec::new()
    }

    /// Get the protocol that the associated type conforms to.
    ///
    /// For AssociatedTypeWitnessTableAccessor, this is the protocol requirement
    /// on the associated type (distinct from the main conformance protocol).
    pub fn associated_type_protocol(&self) -> Option<TypeRef<'ctx>> {
        if self.raw.kind() != NodeKind::AssociatedTypeWitnessTableAccessor {
            return None;
        }
        // The associated type protocol is a Type child that's NOT inside ProtocolConformance
        // It comes after AssocTypePath
        let mut found_assoc_path = false;
        for child in self.raw.children() {
            if child.kind() == NodeKind::AssocTypePath {
                found_assoc_path = true;
            } else if found_assoc_path && child.kind() == NodeKind::Type {
                return Some(TypeRef::new(child.child(0).unwrap_or(child)));
            }
        }
        None
    }
}

impl std::fmt::Debug for WitnessTable<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WitnessTable")
            .field("kind", &self.kind())
            .field("conforming_type", &self.conforming_type())
            .field("protocol", &self.protocol())
            .field("conformance_module", &self.conformance_module())
            .finish()
    }
}

impl std::fmt::Display for WitnessTable<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.raw)
    }
}

/// Information about a protocol conformance.
#[derive(Clone, Copy)]
pub struct ProtocolConformance<'ctx> {
    raw: Node<'ctx>,
}

impl<'ctx> ProtocolConformance<'ctx> {
    /// Create a ProtocolConformance from a raw node.
    pub fn new(raw: Node<'ctx>) -> Self {
        Self { raw }
    }

    /// Get the underlying raw node.
    pub fn raw(&self) -> Node<'ctx> {
        self.raw
    }

    /// Get the conforming type.
    pub fn conforming_type(&self) -> Option<TypeRef<'ctx>> {
        // First Type child is the conforming type
        self.raw.extract_type_ref()
    }

    /// Get the protocol being conformed to.
    pub fn protocol(&self) -> Option<TypeRef<'ctx>> {
        // Second Type child is the protocol
        let mut found_first = false;
        for child in self.raw.children() {
            if child.kind() == NodeKind::Type {
                if found_first {
                    return Some(TypeRef::new(child.child(0).unwrap_or(child)));
                }
                found_first = true;
            }
        }
        None
    }

    /// Get the module where the conformance is declared.
    pub fn module(&self) -> Option<&'ctx str> {
        self.raw
            .child_of_kind(NodeKind::Module)
            .and_then(|c| c.text())
    }
}

impl std::fmt::Debug for ProtocolConformance<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProtocolConformance")
            .field("conforming_type", &self.conforming_type())
            .field("protocol", &self.protocol())
            .field("module", &self.module())
            .finish()
    }
}

/// The kind of witness table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WitnessTableKind {
    /// Protocol witness table.
    Protocol,
    /// Protocol witness table accessor function.
    ProtocolAccessor,
    /// Protocol witness table pattern.
    ProtocolPattern,
    /// Generic protocol witness table.
    GenericProtocol,
    /// Generic protocol witness table instantiation function.
    GenericProtocolInstantiation,
    /// Resilient protocol witness table.
    ResilientProtocol,
    /// Lazy protocol witness table accessor.
    LazyProtocolAccessor,
    /// Lazy protocol witness table cache variable.
    LazyProtocolCacheVariable,
    /// Protocol self-conformance witness table.
    SelfConformance,
    /// Value witness table (for type layout/copying/destroying).
    ValueTable,
    /// A specific value witness function.
    ValueWitness,
    /// Associated type witness table accessor.
    AssociatedTypeAccessor,
    /// Base witness table accessor.
    BaseAccessor,
    /// Concrete protocol conformance.
    ConcreteConformance,
    /// Other witness table kind.
    Other,
}

/// The kind of value witness.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueWitnessKind {
    AllocateBuffer,
    AssignWithCopy,
    AssignWithTake,
    DeallocateBuffer,
    Destroy,
    DestroyBuffer,
    InitializeBufferWithCopyOfBuffer,
    InitializeBufferWithCopy,
    InitializeWithCopy,
    InitializeBufferWithTakeOfBuffer,
    InitializeBufferWithTake,
    InitializeWithTake,
    ProjectBuffer,
    InitializeArrayWithCopy,
    InitializeArrayWithTakeFrontToBack,
    InitializeArrayWithTakeBackToFront,
    StoreExtraInhabitant,
    GetExtraInhabitantIndex,
    GetEnumTag,
    DestructiveProjectEnumData,
    DestructiveInjectEnumTag,
    GetEnumTagSinglePayload,
    StoreEnumTagSinglePayload,
}

impl ValueWitnessKind {
    /// Create from the index value in the mangled symbol.
    pub fn from_index(index: u64) -> Option<Self> {
        match index {
            0 => Some(ValueWitnessKind::AllocateBuffer),
            1 => Some(ValueWitnessKind::AssignWithCopy),
            2 => Some(ValueWitnessKind::AssignWithTake),
            3 => Some(ValueWitnessKind::DeallocateBuffer),
            4 => Some(ValueWitnessKind::Destroy),
            5 => Some(ValueWitnessKind::DestroyBuffer),
            6 => Some(ValueWitnessKind::InitializeBufferWithCopyOfBuffer),
            7 => Some(ValueWitnessKind::InitializeBufferWithCopy),
            8 => Some(ValueWitnessKind::InitializeWithCopy),
            9 => Some(ValueWitnessKind::InitializeBufferWithTakeOfBuffer),
            10 => Some(ValueWitnessKind::InitializeBufferWithTake),
            11 => Some(ValueWitnessKind::InitializeWithTake),
            12 => Some(ValueWitnessKind::ProjectBuffer),
            13 => Some(ValueWitnessKind::InitializeArrayWithCopy),
            14 => Some(ValueWitnessKind::InitializeArrayWithTakeFrontToBack),
            15 => Some(ValueWitnessKind::InitializeArrayWithTakeBackToFront),
            16 => Some(ValueWitnessKind::StoreExtraInhabitant),
            17 => Some(ValueWitnessKind::GetExtraInhabitantIndex),
            18 => Some(ValueWitnessKind::GetEnumTag),
            19 => Some(ValueWitnessKind::DestructiveProjectEnumData),
            20 => Some(ValueWitnessKind::DestructiveInjectEnumTag),
            21 => Some(ValueWitnessKind::GetEnumTagSinglePayload),
            22 => Some(ValueWitnessKind::StoreEnumTagSinglePayload),
            _ => None,
        }
    }

    /// Get a human-readable name for this value witness kind.
    pub fn name(&self) -> &'static str {
        match self {
            ValueWitnessKind::AllocateBuffer => "allocateBuffer",
            ValueWitnessKind::AssignWithCopy => "assignWithCopy",
            ValueWitnessKind::AssignWithTake => "assignWithTake",
            ValueWitnessKind::DeallocateBuffer => "deallocateBuffer",
            ValueWitnessKind::Destroy => "destroy",
            ValueWitnessKind::DestroyBuffer => "destroyBuffer",
            ValueWitnessKind::InitializeBufferWithCopyOfBuffer => {
                "initializeBufferWithCopyOfBuffer"
            }
            ValueWitnessKind::InitializeBufferWithCopy => "initializeBufferWithCopy",
            ValueWitnessKind::InitializeWithCopy => "initializeWithCopy",
            ValueWitnessKind::InitializeBufferWithTakeOfBuffer => {
                "initializeBufferWithTakeOfBuffer"
            }
            ValueWitnessKind::InitializeBufferWithTake => "initializeBufferWithTake",
            ValueWitnessKind::InitializeWithTake => "initializeWithTake",
            ValueWitnessKind::ProjectBuffer => "projectBuffer",
            ValueWitnessKind::InitializeArrayWithCopy => "initializeArrayWithCopy",
            ValueWitnessKind::InitializeArrayWithTakeFrontToBack => {
                "initializeArrayWithTakeFrontToBack"
            }
            ValueWitnessKind::InitializeArrayWithTakeBackToFront => {
                "initializeArrayWithTakeBackToFront"
            }
            ValueWitnessKind::StoreExtraInhabitant => "storeExtraInhabitant",
            ValueWitnessKind::GetExtraInhabitantIndex => "getExtraInhabitantIndex",
            ValueWitnessKind::GetEnumTag => "getEnumTag",
            ValueWitnessKind::DestructiveProjectEnumData => "destructiveProjectEnumData",
            ValueWitnessKind::DestructiveInjectEnumTag => "destructiveInjectEnumTag",
            ValueWitnessKind::GetEnumTagSinglePayload => "getEnumTagSinglePayload",
            ValueWitnessKind::StoreEnumTagSinglePayload => "storeEnumTagSinglePayload",
        }
    }
}

impl WitnessTableKind {
    /// Get a human-readable name for this witness table kind.
    pub fn name(&self) -> &'static str {
        match self {
            WitnessTableKind::Protocol => "protocol witness table",
            WitnessTableKind::ProtocolAccessor => "protocol witness table accessor",
            WitnessTableKind::ProtocolPattern => "protocol witness table pattern",
            WitnessTableKind::GenericProtocol => "generic protocol witness table",
            WitnessTableKind::GenericProtocolInstantiation => {
                "generic protocol witness table instantiation function"
            }
            WitnessTableKind::ResilientProtocol => "resilient protocol witness table",
            WitnessTableKind::LazyProtocolAccessor => "lazy protocol witness table accessor",
            WitnessTableKind::LazyProtocolCacheVariable => {
                "lazy protocol witness table cache variable"
            }
            WitnessTableKind::SelfConformance => "protocol self-conformance witness table",
            WitnessTableKind::ValueTable => "value witness table",
            WitnessTableKind::ValueWitness => "value witness",
            WitnessTableKind::AssociatedTypeAccessor => "associated type witness table accessor",
            WitnessTableKind::BaseAccessor => "base witness table accessor",
            WitnessTableKind::ConcreteConformance => "concrete protocol conformance",
            WitnessTableKind::Other => "witness table",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Symbol;
    use crate::raw::Context;

    #[test]
    fn test_protocol_witness_table() {
        let ctx = Context::new();
        // protocol witness table for Swift.Mirror.DisplayStyle : Swift.Equatable in Swift
        let symbol = Symbol::parse(&ctx, "$ss6MirrorV12DisplayStyleOSQsWP").unwrap();
        assert!(symbol.is_witness_table());
        if let Symbol::WitnessTable(wt) = symbol {
            assert_eq!(wt.kind(), WitnessTableKind::Protocol);
            assert!(wt.conforming_type().is_some());
            assert!(wt.protocol().is_some());
            assert_eq!(wt.conformance_module(), Some("Swift"));
        } else {
            panic!("Expected witness table");
        }
    }
}
