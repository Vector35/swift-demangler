//! Specialization symbol representation.
//!
//! Specializations are pre-compiled versions of generic functions with
//! specific type arguments substituted for performance.

use crate::helpers::NodeExt;
use crate::raw::{Node, NodeKind};
use crate::symbol::Symbol;
use crate::types::TypeRef;

/// A Swift specialization symbol.
#[derive(Clone, Copy)]
pub struct Specialization<'ctx> {
    raw: Node<'ctx>,
}

impl<'ctx> Specialization<'ctx> {
    /// Create a Specialization from a raw node.
    pub fn new(raw: Node<'ctx>) -> Self {
        Self { raw }
    }

    /// Get the underlying raw node.
    pub fn raw(&self) -> Node<'ctx> {
        self.raw
    }

    /// Get the kind of specialization.
    pub fn kind(&self) -> SpecializationKind {
        match self.raw.kind() {
            NodeKind::GenericSpecialization => SpecializationKind::Generic,
            NodeKind::GenericSpecializationNotReAbstracted => {
                SpecializationKind::GenericNotReAbstracted
            }
            NodeKind::GenericSpecializationInResilienceDomain => {
                SpecializationKind::GenericInResilienceDomain
            }
            NodeKind::GenericSpecializationPrespecialized => SpecializationKind::Prespecialized,
            NodeKind::GenericPartialSpecialization => SpecializationKind::Partial,
            NodeKind::GenericPartialSpecializationNotReAbstracted => {
                SpecializationKind::PartialNotReAbstracted
            }
            NodeKind::FunctionSignatureSpecialization => SpecializationKind::FunctionSignature,
            _ => SpecializationKind::Other,
        }
    }

    /// Get the specialization pass ID if present.
    ///
    /// This indicates which optimization pass created the specialization.
    pub fn pass_id(&self) -> Option<u64> {
        self.raw
            .child_of_kind(NodeKind::SpecializationPassID)
            .and_then(|c| c.index())
    }

    /// Get the type arguments used for specialization.
    ///
    /// Returns a list of types that were substituted for generic parameters.
    /// For function signature specializations, use `function_signature_params()` instead.
    pub fn type_arguments(&self) -> Vec<TypeRef<'ctx>> {
        let mut args = Vec::new();
        for child in self.raw.children() {
            if child.kind() == NodeKind::GenericSpecializationParam {
                // GenericSpecializationParam contains a Type child
                for inner in child.children() {
                    if inner.kind() == NodeKind::Type {
                        args.push(TypeRef::new(inner.child(0).unwrap_or(inner)));
                    }
                }
            }
        }
        args
    }

    /// Get the function signature specialization parameters.
    ///
    /// Returns a list of parameter transformations for function signature specializations.
    /// For generic specializations, use `type_arguments()` instead.
    pub fn function_signature_params(&self) -> Vec<FunctionSignatureParam<'ctx>> {
        let mut params = Vec::new();
        for child in self.raw.children() {
            if child.kind() == NodeKind::FunctionSignatureSpecializationParam {
                params.push(FunctionSignatureParam::new(child));
            }
        }
        params
    }

    /// Get the inner symbol that this specialization wraps.
    ///
    /// This is the generic function/method being specialized.
    /// Returns `None` if the inner symbol cannot be determined.
    pub fn inner(&self) -> Option<Symbol<'ctx>> {
        // The inner symbol is a sibling node in the Global parent, not a child
        // We need to get the parent (Global) and find the function child
        // Since we don't have parent pointers, we can't do this directly.
        // Instead, specializations are handled specially in Symbol::from_node
        None
    }

    /// Get the module containing this specialization.
    pub fn module(&self) -> Option<&'ctx str> {
        // Search descendants for Module
        for node in self.raw.descendants() {
            if node.kind() == NodeKind::Module {
                return node.text();
            }
        }
        None
    }
}

impl std::fmt::Debug for Specialization<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = f.debug_struct("Specialization");
        s.field("kind", &self.kind());
        s.field("pass_id", &self.pass_id());
        let type_args = self.type_arguments();
        if !type_args.is_empty() {
            s.field("type_arguments", &type_args);
        }
        let sig_params = self.function_signature_params();
        if !sig_params.is_empty() {
            s.field("function_signature_params", &sig_params);
        }
        s.field("module", &self.module());
        s.finish()
    }
}

impl std::fmt::Display for Specialization<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.raw)
    }
}

/// The kind of specialization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecializationKind {
    /// Full generic specialization.
    Generic,
    /// Generic specialization without re-abstraction.
    GenericNotReAbstracted,
    /// Generic specialization within a resilience domain.
    GenericInResilienceDomain,
    /// Pre-specialized generic (compiled ahead of time).
    Prespecialized,
    /// Partial generic specialization.
    Partial,
    /// Partial specialization without re-abstraction.
    PartialNotReAbstracted,
    /// Function signature specialization (parameter/return type changes).
    FunctionSignature,
    /// Other specialization type.
    Other,
}

impl SpecializationKind {
    /// Get a human-readable name for this specialization kind.
    pub fn name(&self) -> &'static str {
        match self {
            SpecializationKind::Generic => "generic specialization",
            SpecializationKind::GenericNotReAbstracted => {
                "generic specialization (not re-abstracted)"
            }
            SpecializationKind::GenericInResilienceDomain => {
                "generic specialization (resilience domain)"
            }
            SpecializationKind::Prespecialized => "pre-specialization",
            SpecializationKind::Partial => "partial specialization",
            SpecializationKind::PartialNotReAbstracted => {
                "partial specialization (not re-abstracted)"
            }
            SpecializationKind::FunctionSignature => "function signature specialization",
            SpecializationKind::Other => "specialization",
        }
    }
}

/// A parameter in a function signature specialization.
///
/// Describes how a function parameter is transformed in the specialization.
#[derive(Clone, Copy)]
pub struct FunctionSignatureParam<'ctx> {
    raw: Node<'ctx>,
}

impl<'ctx> FunctionSignatureParam<'ctx> {
    /// Create a FunctionSignatureParam from a raw node.
    pub fn new(raw: Node<'ctx>) -> Self {
        Self { raw }
    }

    /// Get the underlying raw node.
    pub fn raw(&self) -> Node<'ctx> {
        self.raw
    }

    /// Get the base kind of parameter transformation.
    pub fn kind(&self) -> FunctionSignatureParamKind {
        for child in self.raw.children() {
            if child.kind() == NodeKind::FunctionSignatureSpecializationParamKind
                && let Some(idx) = child.index()
            {
                return FunctionSignatureParamKind::from_index(idx);
            }
        }
        FunctionSignatureParamKind::Unknown(0)
    }

    /// Get the option flags for this parameter transformation.
    pub fn flags(&self) -> FunctionSignatureParamFlags {
        for child in self.raw.children() {
            if child.kind() == NodeKind::FunctionSignatureSpecializationParamKind
                && let Some(idx) = child.index()
            {
                return FunctionSignatureParamFlags::from_index(idx);
            }
        }
        FunctionSignatureParamFlags::default()
    }

    /// Get all payload texts (e.g., encoding and value for ConstantPropString).
    pub fn payloads(&self) -> Vec<&'ctx str> {
        self.raw
            .children()
            .filter(|c| c.kind() == NodeKind::FunctionSignatureSpecializationParamPayload)
            .filter_map(|c| c.text())
            .collect()
    }

    /// Get the type arguments for this parameter transformation.
    pub fn types(&self) -> Vec<TypeRef<'ctx>> {
        self.raw
            .children()
            .filter(|c| c.kind() == NodeKind::Type)
            .map(|c| TypeRef::new(c.child(0).unwrap_or(c)))
            .collect()
    }
}

impl std::fmt::Debug for FunctionSignatureParam<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = f.debug_struct("FunctionSignatureParam");
        s.field("kind", &self.kind());
        let payloads = self.payloads();
        if !payloads.is_empty() {
            s.field("payloads", &payloads);
        }
        let types = self.types();
        if !types.is_empty() {
            s.field("types", &types);
        }
        s.finish()
    }
}

impl std::fmt::Display for FunctionSignatureParam<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.raw)
    }
}

/// The base kind of function signature parameter transformation.
///
/// This represents the low bits (0-5) of the param kind value.
/// Additional flags (Dead, OwnedToGuaranteed, etc.) can be combined.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FunctionSignatureParamKind {
    /// Function constant propagation.
    ConstantPropFunction,
    /// Global constant propagation.
    ConstantPropGlobal,
    /// Integer constant propagation.
    ConstantPropInteger,
    /// Float constant propagation.
    ConstantPropFloat,
    /// String constant propagation.
    ConstantPropString,
    /// Closure parameter was propagated/inlined.
    ClosureProp,
    /// Box converted to value.
    BoxToValue,
    /// Box converted to stack allocation.
    BoxToStack,
    /// In-out parameter converted to out.
    InOutToOut,
    /// KeyPath constant propagation.
    ConstantPropKeyPath,
    /// Unknown base kind.
    Unknown(u64),
}

impl FunctionSignatureParamKind {
    /// Convert from the raw index value (low 6 bits) to a base param kind.
    fn from_index(idx: u64) -> Self {
        // Extract low 6 bits for base kind
        let base = idx & 0x3F;
        match base {
            0 => FunctionSignatureParamKind::ConstantPropFunction,
            1 => FunctionSignatureParamKind::ConstantPropGlobal,
            2 => FunctionSignatureParamKind::ConstantPropInteger,
            3 => FunctionSignatureParamKind::ConstantPropFloat,
            4 => FunctionSignatureParamKind::ConstantPropString,
            5 => FunctionSignatureParamKind::ClosureProp,
            6 => FunctionSignatureParamKind::BoxToValue,
            7 => FunctionSignatureParamKind::BoxToStack,
            8 => FunctionSignatureParamKind::InOutToOut,
            9 => FunctionSignatureParamKind::ConstantPropKeyPath,
            _ => FunctionSignatureParamKind::Unknown(base),
        }
    }

    /// Get a human-readable name for this param kind.
    pub fn name(&self) -> &'static str {
        match self {
            FunctionSignatureParamKind::ConstantPropFunction => "constant prop (function)",
            FunctionSignatureParamKind::ConstantPropGlobal => "constant prop (global)",
            FunctionSignatureParamKind::ConstantPropInteger => "constant prop (integer)",
            FunctionSignatureParamKind::ConstantPropFloat => "constant prop (float)",
            FunctionSignatureParamKind::ConstantPropString => "constant prop (string)",
            FunctionSignatureParamKind::ClosureProp => "closure propagated",
            FunctionSignatureParamKind::BoxToValue => "box to value",
            FunctionSignatureParamKind::BoxToStack => "box to stack",
            FunctionSignatureParamKind::InOutToOut => "inout to out",
            FunctionSignatureParamKind::ConstantPropKeyPath => "constant prop (keypath)",
            FunctionSignatureParamKind::Unknown(_) => "unknown",
        }
    }
}

/// Option flags that can be combined with the base param kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FunctionSignatureParamFlags {
    /// Parameter was found dead (unused) and removed.
    pub dead: bool,
    /// Owned parameter converted to guaranteed (borrowed).
    pub owned_to_guaranteed: bool,
    /// Exploded (SROA - Scalar Replacement of Aggregates) - struct/tuple split into fields.
    pub exploded: bool,
    /// Guaranteed parameter converted to owned.
    pub guaranteed_to_owned: bool,
    /// Existential parameter specialized to concrete type.
    pub existential_to_generic: bool,
}

impl FunctionSignatureParamFlags {
    /// Extract flags from the raw index value (bits 6+).
    fn from_index(idx: u64) -> Self {
        Self {
            dead: (idx & (1 << 6)) != 0,
            owned_to_guaranteed: (idx & (1 << 7)) != 0,
            exploded: (idx & (1 << 8)) != 0,
            guaranteed_to_owned: (idx & (1 << 9)) != 0,
            existential_to_generic: (idx & (1 << 10)) != 0,
        }
    }

    /// Returns true if any flag is set.
    pub fn any(&self) -> bool {
        self.dead
            || self.owned_to_guaranteed
            || self.exploded
            || self.guaranteed_to_owned
            || self.existential_to_generic
    }

    /// Get a list of flag names that are set.
    pub fn names(&self) -> Vec<&'static str> {
        let mut names = Vec::new();
        if self.dead {
            names.push("dead");
        }
        if self.owned_to_guaranteed {
            names.push("owned to guaranteed");
        }
        if self.exploded {
            names.push("exploded");
        }
        if self.guaranteed_to_owned {
            names.push("guaranteed to owned");
        }
        if self.existential_to_generic {
            names.push("existential to generic");
        }
        names
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::raw::Context;
    use crate::symbol::SpecializedSymbol;

    #[test]
    fn test_prespecialization() {
        let ctx = Context::new();
        // generic pre-specialization <Swift.AnyObject> of Array._createNewBuffer
        let symbol = Symbol::parse(
            &ctx,
            "$sSa16_createNewBuffer14bufferIsUnique15minimumCapacity13growForAppendySb_SiSbtFyXl_Ts5",
        )
        .unwrap();
        assert!(symbol.is_specialization());
        if let Symbol::Specialization(SpecializedSymbol {
            specialization,
            inner,
        }) = symbol
        {
            assert_eq!(specialization.kind(), SpecializationKind::Prespecialized);
            assert_eq!(specialization.pass_id(), Some(5));
            let type_args = specialization.type_arguments();
            assert_eq!(type_args.len(), 1);
            // Inner should be a function
            assert!(inner.is_function());
        } else {
            panic!("Expected specialization");
        }
    }

    #[test]
    fn test_function_signature_specialization() {
        let ctx = Context::new();
        // function signature specialization with closure propagation
        let symbol = Symbol::parse(
            &ctx,
            "_TTSf1cl35_TFF7specgen6callerFSiT_U_FTSiSi_T_Si___TF7specgen12take_closureFFTSiSi_T_T_",
        )
        .unwrap();
        assert!(symbol.is_specialization());
        if let Symbol::Specialization(SpecializedSymbol {
            specialization,
            inner,
        }) = symbol
        {
            assert_eq!(specialization.kind(), SpecializationKind::FunctionSignature);
            assert_eq!(specialization.pass_id(), Some(1));

            // Should have function signature params, not type arguments
            let type_args = specialization.type_arguments();
            assert!(type_args.is_empty());

            let func_params = specialization.function_signature_params();
            assert_eq!(func_params.len(), 1);

            let param = &func_params[0];
            assert_eq!(param.kind(), FunctionSignatureParamKind::ClosureProp);
            assert!(!param.flags().any()); // No flags set

            // Should have payload (the closure mangled name)
            assert!(!param.payloads().is_empty());

            // Should have one type argument (Int)
            let types = param.types();
            assert_eq!(types.len(), 1);

            // Inner should be a function
            assert!(inner.is_function());
        } else {
            panic!("Expected specialization");
        }
    }

    #[test]
    fn test_nested_function_signature_specialization() {
        let ctx = Context::new();
        // Nested specialization: spec of spec of constructor
        let symbol = Symbol::parse(
            &ctx,
            "_TTSf2dgs___TTSf2s_d___TFVs17_LegacyStringCoreCfVs13_StringBufferS_",
        )
        .unwrap();
        assert!(symbol.is_specialization());

        // Outer specialization
        if let Symbol::Specialization(SpecializedSymbol {
            specialization,
            inner,
        }) = symbol
        {
            assert_eq!(specialization.kind(), SpecializationKind::FunctionSignature);

            let func_params = specialization.function_signature_params();
            assert_eq!(func_params.len(), 1);
            let flags = func_params[0].flags();
            assert!(flags.dead);
            assert!(flags.owned_to_guaranteed);
            assert!(flags.exploded);

            // Inner should also be a specialization
            assert!(inner.is_specialization());
            if let Symbol::Specialization(SpecializedSymbol {
                specialization: inner_spec,
                inner: innermost,
            }) = inner.as_ref()
            {
                assert_eq!(inner_spec.kind(), SpecializationKind::FunctionSignature);

                let inner_params = inner_spec.function_signature_params();
                assert_eq!(inner_params.len(), 2);
                assert!(inner_params[0].flags().exploded);
                assert!(inner_params[1].flags().dead);

                // Innermost should be a constructor
                assert!(innermost.is_constructor());
            } else {
                panic!("Expected inner specialization");
            }
        } else {
            panic!("Expected specialization");
        }
    }
}
