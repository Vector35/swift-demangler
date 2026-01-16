//! Thunk symbol representation.
//!
//! Thunks are wrapper functions that adapt calling conventions, perform
//! protocol dispatch, or handle partial application.

use crate::function::Function;
use crate::helpers::{HasModule, NodeExt};
use crate::raw::{Node, NodeKind};
use crate::symbol::Symbol;
use crate::types::{GenericSignature, ImplFunctionType, TypeRef};
use crate::witness_table::ProtocolConformance;

/// A Swift thunk symbol.
pub enum Thunk<'ctx> {
    /// Reabstraction thunk - converts between calling conventions.
    Reabstraction(ReabstractionThunk<'ctx>),
    /// Protocol witness thunk - adapts concrete impl to protocol requirement.
    ProtocolWitness(ProtocolWitnessThunk<'ctx>),
    /// AutoDiff thunk - handles automatic differentiation.
    AutoDiff(AutoDiffThunk<'ctx>),
    /// Dispatch thunk - looks up implementation in witness table or vtable.
    Dispatch {
        inner: Box<Symbol<'ctx>>,
        kind: DispatchKind,
        raw: Node<'ctx>,
    },
    /// Partial application thunk - unpacks captured context.
    PartialApply {
        inner: Option<Box<Symbol<'ctx>>>,
        is_objc: bool,
        raw: Node<'ctx>,
    },
    /// Other thunk types.
    Other {
        kind: OtherThunkKind,
        inner: Option<Box<Symbol<'ctx>>>,
        raw: Node<'ctx>,
    },
}

impl std::fmt::Debug for Thunk<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Reabstraction(t) => f.debug_tuple("Reabstraction").field(t).finish(),
            Self::ProtocolWitness(t) => f.debug_tuple("ProtocolWitness").field(t).finish(),
            Self::AutoDiff(t) => f.debug_tuple("AutoDiff").field(t).finish(),
            Self::Dispatch { inner, kind, .. } => f
                .debug_struct("Dispatch")
                .field("inner", inner)
                .field("kind", kind)
                .finish(),
            Self::PartialApply { inner, is_objc, .. } => f
                .debug_struct("PartialApply")
                .field("inner", inner)
                .field("is_objc", is_objc)
                .finish(),
            Self::Other { kind, inner, .. } => f
                .debug_struct("Other")
                .field("kind", kind)
                .field("inner", inner)
                .finish(),
        }
    }
}

impl<'ctx> Thunk<'ctx> {
    /// Create a Thunk from a raw node.
    pub fn new(raw: Node<'ctx>) -> Self {
        match raw.kind() {
            // Reabstraction thunks
            NodeKind::ReabstractionThunk
            | NodeKind::ReabstractionThunkHelper
            | NodeKind::ReabstractionThunkHelperWithSelf
            | NodeKind::ReabstractionThunkHelperWithGlobalActor
            | NodeKind::AutoDiffSelfReorderingReabstractionThunk => {
                Self::Reabstraction(ReabstractionThunk::new(raw))
            }

            // Protocol witness thunks
            NodeKind::ProtocolWitness | NodeKind::ProtocolSelfConformanceWitness => {
                Self::ProtocolWitness(ProtocolWitnessThunk::new(raw))
            }

            // AutoDiff thunks
            NodeKind::AutoDiffSubsetParametersThunk | NodeKind::AutoDiffDerivativeVTableThunk => {
                Self::AutoDiff(AutoDiffThunk::new(raw))
            }

            // Dispatch thunks
            NodeKind::DispatchThunk => Self::Dispatch {
                inner: Box::new(
                    raw.child(0)
                        .map(Symbol::classify_node)
                        .unwrap_or(Symbol::Other(raw)),
                ),
                kind: DispatchKind::Protocol,
                raw,
            },
            NodeKind::VTableThunk => Self::Dispatch {
                inner: Box::new(
                    raw.child(0)
                        .map(Symbol::classify_node)
                        .unwrap_or(Symbol::Other(raw)),
                ),
                kind: DispatchKind::VTable,
                raw,
            },
            NodeKind::DistributedThunk => {
                // DistributedThunk may have the function as a child or in descendants
                let inner = raw.child(0).map(Symbol::classify_node).or_else(|| {
                    // Search descendants for a Function node
                    raw.descendants()
                        .find(|d| d.kind() == NodeKind::Function)
                        .map(Symbol::classify_node)
                });
                Self::Dispatch {
                    inner: Box::new(inner.unwrap_or(Symbol::Other(raw))),
                    kind: DispatchKind::Distributed,
                    raw,
                }
            }

            // Partial apply thunks
            NodeKind::PartialApplyForwarder => Self::PartialApply {
                inner: find_inner_symbol(raw),
                is_objc: false,
                raw,
            },
            NodeKind::PartialApplyObjCForwarder => Self::PartialApply {
                inner: find_inner_symbol(raw),
                is_objc: true,
                raw,
            },

            // Other thunks
            NodeKind::CurryThunk => Self::Other {
                kind: OtherThunkKind::Curry,
                inner: raw.child(0).map(|c| Box::new(Symbol::classify_node(c))),
                raw,
            },
            NodeKind::KeyPathGetterThunkHelper => Self::Other {
                kind: OtherThunkKind::KeyPathGetter,
                inner: raw.child(0).map(|c| Box::new(Symbol::classify_node(c))),
                raw,
            },
            NodeKind::KeyPathSetterThunkHelper => Self::Other {
                kind: OtherThunkKind::KeyPathSetter,
                inner: raw.child(0).map(|c| Box::new(Symbol::classify_node(c))),
                raw,
            },
            NodeKind::KeyPathUnappliedMethodThunkHelper
            | NodeKind::KeyPathAppliedMethodThunkHelper => Self::Other {
                kind: OtherThunkKind::KeyPathMethod,
                inner: raw.child(0).map(|c| Box::new(Symbol::classify_node(c))),
                raw,
            },
            NodeKind::KeyPathEqualsThunkHelper => Self::Other {
                kind: OtherThunkKind::KeyPathEquals,
                inner: raw.child(0).map(|c| Box::new(Symbol::classify_node(c))),
                raw,
            },
            NodeKind::KeyPathHashThunkHelper => Self::Other {
                kind: OtherThunkKind::KeyPathHash,
                inner: raw.child(0).map(|c| Box::new(Symbol::classify_node(c))),
                raw,
            },
            NodeKind::BackDeploymentThunk => Self::Other {
                kind: OtherThunkKind::BackDeployment,
                inner: raw.child(0).map(|c| Box::new(Symbol::classify_node(c))),
                raw,
            },
            NodeKind::BackDeploymentFallback => Self::Other {
                kind: OtherThunkKind::BackDeploymentFallback,
                inner: raw.child(0).map(|c| Box::new(Symbol::classify_node(c))),
                raw,
            },
            NodeKind::MergedFunction => Self::Other {
                kind: OtherThunkKind::Merged,
                inner: find_inner_symbol_skip_metadata(raw),
                raw,
            },
            NodeKind::InlinedGenericFunction => Self::Other {
                kind: OtherThunkKind::InlinedGeneric,
                inner: find_inner_symbol_skip_metadata(raw),
                raw,
            },
            _ => Self::Other {
                kind: OtherThunkKind::Unknown,
                inner: raw.child(0).map(|c| Box::new(Symbol::classify_node(c))),
                raw,
            },
        }
    }

    /// Create a Thunk marker with an already-classified inner symbol.
    ///
    /// This is used for sibling patterns where the inner symbol is the next sibling
    /// in Global, not a child of the marker node.
    pub fn new_marker(raw: Node<'ctx>, inner: Symbol<'ctx>) -> Self {
        match raw.kind() {
            NodeKind::DistributedThunk => Self::Dispatch {
                inner: Box::new(inner),
                kind: DispatchKind::Distributed,
                raw,
            },
            NodeKind::MergedFunction => Self::Other {
                kind: OtherThunkKind::Merged,
                inner: Some(Box::new(inner)),
                raw,
            },
            NodeKind::InlinedGenericFunction => Self::Other {
                kind: OtherThunkKind::InlinedGeneric,
                inner: Some(Box::new(inner)),
                raw,
            },
            _ => Self::Other {
                kind: OtherThunkKind::Unknown,
                inner: Some(Box::new(inner)),
                raw,
            },
        }
    }

    /// Get the underlying raw node.
    pub fn raw(&self) -> Node<'ctx> {
        match self {
            Self::Reabstraction(t) => t.raw,
            Self::ProtocolWitness(t) => t.raw,
            Self::AutoDiff(t) => t.raw,
            Self::Dispatch { raw, .. } => *raw,
            Self::PartialApply { raw, .. } => *raw,
            Self::Other { raw, .. } => *raw,
        }
    }

    /// Get the module containing this thunk.
    pub fn module(&self) -> Option<&'ctx str> {
        match self {
            Self::Reabstraction(t) => t.module(),
            Self::ProtocolWitness(t) => t.module(),
            Self::AutoDiff(t) => t.module(),
            Self::Dispatch { raw, .. } => find_module_in_descendants(*raw),
            Self::PartialApply { raw, .. } => find_module_in_descendants(*raw),
            Self::Other { raw, .. } => find_module_in_descendants(*raw),
        }
    }

    /// Get a human-readable description of this thunk kind.
    pub fn kind_name(&self) -> &'static str {
        match self {
            Self::Reabstraction(_) => "reabstraction thunk",
            Self::ProtocolWitness(t) => {
                if t.is_self_conformance {
                    "protocol self-conformance witness"
                } else {
                    "protocol witness"
                }
            }
            Self::AutoDiff(t) => t.kind.name(),
            Self::Dispatch { kind, .. } => kind.name(),
            Self::PartialApply { is_objc, .. } => {
                if *is_objc {
                    "partial apply ObjC forwarder"
                } else {
                    "partial apply forwarder"
                }
            }
            Self::Other { kind, .. } => kind.name(),
        }
    }
}

impl std::fmt::Display for Thunk<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.raw())
    }
}

fn find_module_in_descendants(node: Node<'_>) -> Option<&'_ str> {
    for desc in node.descendants() {
        if desc.kind() == NodeKind::Module {
            return desc.text();
        }
    }
    None
}

/// Find the inner symbol in a thunk node, skipping attribute nodes.
fn find_inner_symbol<'ctx>(raw: Node<'ctx>) -> Option<Box<Symbol<'ctx>>> {
    for child in raw.children() {
        // Skip attribute nodes - look for the actual symbol
        match child.kind() {
            NodeKind::ObjCAttribute | NodeKind::NonObjCAttribute | NodeKind::DynamicAttribute => {
                continue;
            }
            _ => return Some(Box::new(Symbol::classify_node(child))),
        }
    }
    None
}

/// Find the inner symbol, skipping metadata nodes like SpecializationPassID.
fn find_inner_symbol_skip_metadata<'ctx>(raw: Node<'ctx>) -> Option<Box<Symbol<'ctx>>> {
    for child in raw.children() {
        // Skip metadata nodes - look for the actual symbol
        match child.kind() {
            NodeKind::SpecializationPassID | NodeKind::Number | NodeKind::Index => continue,
            _ => return Some(Box::new(Symbol::classify_node(child))),
        }
    }
    None
}

/// A reabstraction thunk that converts between calling conventions.
#[derive(Clone, Copy)]
pub struct ReabstractionThunk<'ctx> {
    raw: Node<'ctx>,
}

impl<'ctx> ReabstractionThunk<'ctx> {
    fn new(raw: Node<'ctx>) -> Self {
        Self { raw }
    }

    /// Get the target function type (the "to" type).
    pub fn target(&self) -> Option<TypeRef<'ctx>> {
        self.raw.extract_type_ref()
    }

    /// Get the source function type (the "from" type).
    pub fn source(&self) -> Option<TypeRef<'ctx>> {
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

    /// Get the generic signature if this is a generic reabstraction thunk.
    pub fn generic_signature(&self) -> Option<GenericSignature<'ctx>> {
        self.raw
            .child_of_kind(NodeKind::DependentGenericSignature)
            .map(GenericSignature::new)
    }

    /// Get the module containing this thunk.
    pub fn module(&self) -> Option<&'ctx str> {
        find_module_in_descendants(self.raw)
    }
}

impl std::fmt::Debug for ReabstractionThunk<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = f.debug_struct("ReabstractionThunk");
        s.field("target", &self.target());
        s.field("source", &self.source());
        if let Some(sig) = self.generic_signature() {
            s.field("generic_signature", &sig);
        }
        s.field("module", &self.module());
        s.finish()
    }
}

/// A protocol witness thunk that adapts a concrete implementation to a protocol requirement.
#[derive(Clone, Copy)]
pub struct ProtocolWitnessThunk<'ctx> {
    raw: Node<'ctx>,
    is_self_conformance: bool,
}

impl<'ctx> ProtocolWitnessThunk<'ctx> {
    fn new(raw: Node<'ctx>) -> Self {
        Self {
            raw,
            is_self_conformance: raw.kind() == NodeKind::ProtocolSelfConformanceWitness,
        }
    }

    /// Get the protocol conformance.
    pub fn conformance(&self) -> Option<ProtocolConformance<'ctx>> {
        self.raw
            .child_of_kind(NodeKind::ProtocolConformance)
            .map(ProtocolConformance::new)
    }

    /// Get the inner function being witnessed.
    pub fn inner(&self) -> Option<Symbol<'ctx>> {
        for child in self.raw.children() {
            if child.kind() == NodeKind::Function {
                return Some(Symbol::classify_node(child));
            }
        }
        // Fall back to first non-conformance child
        for child in self.raw.children() {
            if child.kind() != NodeKind::ProtocolConformance {
                return Some(Symbol::classify_node(child));
            }
        }
        None
    }

    /// Check if this is a self-conformance witness.
    pub fn is_self_conformance(&self) -> bool {
        self.is_self_conformance
    }

    /// Get the module containing this thunk.
    pub fn module(&self) -> Option<&'ctx str> {
        self.conformance()
            .and_then(|c| c.module())
            .or_else(|| find_module_in_descendants(self.raw))
    }
}

impl std::fmt::Debug for ProtocolWitnessThunk<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProtocolWitnessThunk")
            .field("is_self_conformance", &self.is_self_conformance)
            .field("conformance", &self.conformance())
            .field("inner", &self.inner())
            .field("module", &self.module())
            .finish()
    }
}

/// An automatic differentiation thunk.
#[derive(Clone, Copy)]
pub struct AutoDiffThunk<'ctx> {
    raw: Node<'ctx>,
    kind: AutoDiffThunkKind,
}

impl<'ctx> AutoDiffThunk<'ctx> {
    fn new(raw: Node<'ctx>) -> Self {
        let kind = match raw.kind() {
            NodeKind::AutoDiffSubsetParametersThunk => AutoDiffThunkKind::SubsetParameters,
            NodeKind::AutoDiffDerivativeVTableThunk => AutoDiffThunkKind::DerivativeVTable,
            _ => AutoDiffThunkKind::Unknown,
        };
        Self { raw, kind }
    }

    /// Get the kind of autodiff thunk.
    pub fn kind(&self) -> AutoDiffThunkKind {
        self.kind
    }

    /// Get the original function being differentiated.
    pub fn function(&self) -> Option<Function<'ctx>> {
        self.raw
            .child_of_kind(NodeKind::Function)
            .map(Function::new)
    }

    /// Get the autodiff function kind (forward, reverse, etc.).
    pub fn autodiff_function_kind(&self) -> Option<u64> {
        self.raw
            .child_of_kind(NodeKind::AutoDiffFunctionKind)
            .and_then(|c| c.index())
    }

    /// Get the function type of this thunk (with full SIL conventions).
    pub fn function_type(&self) -> Option<ImplFunctionType<'ctx>> {
        for child in self.raw.children() {
            if let Some(inner) = child.unwrap_if_kind(NodeKind::Type)
                && inner.kind() == NodeKind::ImplFunctionType
            {
                return Some(ImplFunctionType::new(inner));
            }
        }
        None
    }

    /// Get the parameter indices (as a string like "SSS" where S=selected, U=unselected).
    pub fn parameter_indices(&self) -> Option<&'ctx str> {
        let mut found_first = false;
        for child in self.raw.children() {
            if child.kind() == NodeKind::IndexSubset {
                if found_first {
                    return child.text();
                }
                found_first = true;
            }
        }
        None
    }

    /// Get the result indices (as a string like "S" where S=selected, U=unselected).
    pub fn result_indices(&self) -> Option<&'ctx str> {
        for child in self.raw.children() {
            if child.kind() == NodeKind::IndexSubset {
                return child.text();
            }
        }
        None
    }

    /// Get the "to" parameter indices for subset parameters thunks.
    pub fn to_parameter_indices(&self) -> Option<&'ctx str> {
        let mut count = 0;
        for child in self.raw.children() {
            if child.kind() == NodeKind::IndexSubset {
                count += 1;
                if count == 3 {
                    return child.text();
                }
            }
        }
        None
    }

    /// Get the module containing this thunk.
    pub fn module(&self) -> Option<&'ctx str> {
        self.function()
            .and_then(|f| f.module())
            .or_else(|| find_module_in_descendants(self.raw))
    }
}

impl std::fmt::Debug for AutoDiffThunk<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = f.debug_struct("AutoDiffThunk");
        s.field("kind", &self.kind);
        s.field("autodiff_function_kind", &self.autodiff_function_kind());
        s.field("function", &self.function());
        s.field("function_type", &self.function_type());
        s.field("parameter_indices", &self.parameter_indices());
        s.field("result_indices", &self.result_indices());
        if self.kind == AutoDiffThunkKind::SubsetParameters {
            s.field("to_parameter_indices", &self.to_parameter_indices());
        }
        s.field("module", &self.module());
        s.finish()
    }
}

/// The kind of autodiff thunk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutoDiffThunkKind {
    /// Subset parameters thunk.
    SubsetParameters,
    /// Derivative vtable thunk.
    DerivativeVTable,
    /// Unknown autodiff kind.
    Unknown,
}

impl AutoDiffThunkKind {
    /// Get a human-readable name for this autodiff kind.
    pub fn name(&self) -> &'static str {
        match self {
            Self::SubsetParameters => "autodiff subset parameters thunk",
            Self::DerivativeVTable => "autodiff derivative vtable thunk",
            Self::Unknown => "autodiff thunk",
        }
    }
}

/// The kind of dispatch thunk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DispatchKind {
    /// Protocol dispatch - looks up in witness table.
    Protocol,
    /// VTable dispatch - looks up in class vtable.
    VTable,
    /// Distributed actor dispatch.
    Distributed,
}

impl DispatchKind {
    /// Get a human-readable name for this dispatch kind.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Protocol => "dispatch thunk",
            Self::VTable => "vtable thunk",
            Self::Distributed => "distributed thunk",
        }
    }
}

/// Other thunk kinds that don't need specialized handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OtherThunkKind {
    /// Curry thunk.
    Curry,
    /// KeyPath getter thunk.
    KeyPathGetter,
    /// KeyPath setter thunk.
    KeyPathSetter,
    /// KeyPath method thunk.
    KeyPathMethod,
    /// KeyPath equality thunk.
    KeyPathEquals,
    /// KeyPath hash thunk.
    KeyPathHash,
    /// Back deployment thunk.
    BackDeployment,
    /// Back deployment fallback.
    BackDeploymentFallback,
    /// Merged function.
    Merged,
    /// Inlined generic function.
    InlinedGeneric,
    /// Unknown thunk.
    Unknown,
}

impl OtherThunkKind {
    /// Get a human-readable name for this thunk kind.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Curry => "curry thunk",
            Self::KeyPathGetter => "keypath getter thunk",
            Self::KeyPathSetter => "keypath setter thunk",
            Self::KeyPathMethod => "keypath method thunk",
            Self::KeyPathEquals => "keypath equals thunk",
            Self::KeyPathHash => "keypath hash thunk",
            Self::BackDeployment => "back deployment thunk",
            Self::BackDeploymentFallback => "back deployment fallback",
            Self::Merged => "merged function",
            Self::InlinedGeneric => "inlined generic function",
            Self::Unknown => "thunk",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::raw::Context;

    #[test]
    fn test_dispatch_thunk() {
        let ctx = Context::new();
        // dispatch thunk of Swift.SetAlgebra.init<A>(__owned A1) -> A
        let symbol = Symbol::parse(
            &ctx,
            "$ss10SetAlgebraPyxqd__ncSTRd__7ElementQyd__ACRtzlufCTj",
        )
        .unwrap();
        assert!(symbol.is_thunk());
        if let Symbol::Thunk(Thunk::Dispatch { inner, kind, .. }) = symbol {
            assert_eq!(kind, DispatchKind::Protocol);
            assert!(inner.is_constructor());
        } else {
            panic!("Expected dispatch thunk");
        }
    }

    #[test]
    fn test_protocol_witness() {
        let ctx = Context::new();
        // protocol witness for call_protocol.P.foo() -> Swift.Int in conformance call_protocol.C : call_protocol.P
        let symbol = Symbol::parse(&ctx, "_TTWC13call_protocol1CS_1PS_FS1_3foofT_Si").unwrap();
        assert!(symbol.is_thunk());
        if let Symbol::Thunk(Thunk::ProtocolWitness(thunk)) = symbol {
            // Check conformance
            let conformance = thunk.conformance();
            assert!(conformance.is_some());
            let conformance = conformance.unwrap();
            assert!(conformance.conforming_type().is_some());
            assert!(conformance.protocol().is_some());
            assert_eq!(conformance.module(), Some("call_protocol"));

            // Check inner function
            let inner = thunk.inner();
            assert!(inner.is_some());
            if let Some(Symbol::Function(func)) = inner {
                assert_eq!(func.name(), Some("foo"));
                assert_eq!(func.containing_type(), Some("P"));
            } else {
                panic!("Expected function as inner symbol");
            }
        } else {
            panic!("Expected protocol witness thunk");
        }
    }
}
