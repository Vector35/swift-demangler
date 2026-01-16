//! Automatic differentiation symbol representation.
//!
//! These symbols represent Swift's automatic differentiation features
//! used in machine learning and scientific computing.

use crate::helpers::{HasModule, NodeExt};
use crate::raw::{Node, NodeKind};

/// A Swift automatic differentiation symbol.
#[derive(Clone, Copy)]
pub struct AutoDiff<'ctx> {
    raw: Node<'ctx>,
}

impl<'ctx> AutoDiff<'ctx> {
    /// Create an AutoDiff from a raw node.
    pub fn new(raw: Node<'ctx>) -> Self {
        Self { raw }
    }

    /// Get the underlying raw node.
    pub fn raw(&self) -> Node<'ctx> {
        self.raw
    }

    /// Get the kind of auto-diff symbol.
    pub fn kind(&self) -> AutoDiffKind {
        match self.raw.kind() {
            NodeKind::AutoDiffFunction => AutoDiffKind::Function,
            NodeKind::DifferentiabilityWitness => AutoDiffKind::DifferentiabilityWitness,
            NodeKind::AutoDiffDerivativeVTableThunk => AutoDiffKind::DerivativeVTableThunk,
            NodeKind::AutoDiffSubsetParametersThunk => AutoDiffKind::SubsetParametersThunk,
            NodeKind::AutoDiffSelfReorderingReabstractionThunk => {
                AutoDiffKind::SelfReorderingReabstractionThunk
            }
            _ => AutoDiffKind::Other,
        }
    }

    /// Get the inner function being differentiated, if any.
    pub fn inner_function(&self) -> Option<crate::function::Function<'ctx>> {
        self.raw
            .child_of_kind(NodeKind::Function)
            .map(crate::function::Function::new)
    }

    /// Get the module containing this auto-diff symbol.
    pub fn module(&self) -> Option<&'ctx str> {
        // Try inner function first
        if let Some(func) = self.inner_function() {
            return func.module();
        }
        // Search descendants
        for node in self.raw.descendants() {
            if node.kind() == NodeKind::Module {
                return node.text();
            }
        }
        None
    }
}

impl std::fmt::Debug for AutoDiff<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AutoDiff")
            .field("kind", &self.kind())
            .field("inner_function", &self.inner_function())
            .field("module", &self.module())
            .finish()
    }
}

impl std::fmt::Display for AutoDiff<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.raw)
    }
}

/// The kind of automatic differentiation symbol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutoDiffKind {
    /// An auto-diff derivative function.
    Function,
    /// A differentiability witness.
    DifferentiabilityWitness,
    /// A derivative vtable thunk.
    DerivativeVTableThunk,
    /// A subset parameters thunk.
    SubsetParametersThunk,
    /// A self-reordering reabstraction thunk.
    SelfReorderingReabstractionThunk,
    /// Other auto-diff symbol.
    Other,
}

impl AutoDiffKind {
    /// Get a human-readable name for this auto-diff kind.
    pub fn name(&self) -> &'static str {
        match self {
            AutoDiffKind::Function => "auto-diff function",
            AutoDiffKind::DifferentiabilityWitness => "differentiability witness",
            AutoDiffKind::DerivativeVTableThunk => "derivative vtable thunk",
            AutoDiffKind::SubsetParametersThunk => "subset parameters thunk",
            AutoDiffKind::SelfReorderingReabstractionThunk => "self-reordering reabstraction thunk",
            AutoDiffKind::Other => "auto-diff symbol",
        }
    }
}
