//! Outlined operation symbol representation.
//!
//! Outlined operations are compiler-generated helpers for common operations
//! that have been "outlined" (pulled out into separate functions) for code
//! size optimization or bridging purposes.

use crate::helpers::NodeExt;
use crate::raw::{Node, NodeKind};

/// A Swift outlined operation symbol.
#[derive(Clone, Copy)]
pub struct Outlined<'ctx> {
    raw: Node<'ctx>,
}

impl<'ctx> Outlined<'ctx> {
    /// Create an Outlined from a raw node.
    pub fn new(raw: Node<'ctx>) -> Self {
        Self { raw }
    }

    /// Get the underlying raw node.
    pub fn raw(&self) -> Node<'ctx> {
        self.raw
    }

    /// Get the kind of outlined operation.
    pub fn kind(&self) -> OutlinedKind {
        match self.raw.kind() {
            NodeKind::OutlinedBridgedMethod => OutlinedKind::BridgedMethod,
            NodeKind::OutlinedVariable => OutlinedKind::Variable,
            NodeKind::OutlinedCopy => OutlinedKind::Copy,
            NodeKind::OutlinedConsume => OutlinedKind::Consume,
            NodeKind::OutlinedRetain => OutlinedKind::Retain,
            NodeKind::OutlinedRelease => OutlinedKind::Release,
            NodeKind::OutlinedInitializeWithTake => OutlinedKind::InitializeWithTake,
            NodeKind::OutlinedInitializeWithCopy => OutlinedKind::InitializeWithCopy,
            NodeKind::OutlinedAssignWithTake => OutlinedKind::AssignWithTake,
            NodeKind::OutlinedAssignWithCopy => OutlinedKind::AssignWithCopy,
            NodeKind::OutlinedDestroy => OutlinedKind::Destroy,
            NodeKind::OutlinedInitializeWithCopyNoValueWitness => {
                OutlinedKind::InitializeWithCopyNoValueWitness
            }
            NodeKind::OutlinedInitializeWithTakeNoValueWitness => {
                OutlinedKind::InitializeWithTakeNoValueWitness
            }
            NodeKind::OutlinedAssignWithCopyNoValueWitness => {
                OutlinedKind::AssignWithCopyNoValueWitness
            }
            NodeKind::OutlinedAssignWithTakeNoValueWitness => {
                OutlinedKind::AssignWithTakeNoValueWitness
            }
            NodeKind::OutlinedDestroyNoValueWitness => OutlinedKind::DestroyNoValueWitness,
            NodeKind::OutlinedReadOnlyObject => OutlinedKind::ReadOnlyObject,
            _ => OutlinedKind::Other,
        }
    }

    /// Get the index of this outlined operation (for variables, copies, etc.).
    pub fn index(&self) -> Option<u64> {
        // The index is stored directly on the node, not as a child
        self.raw.index()
    }

    /// Get the module containing this outlined operation.
    pub fn module(&self) -> Option<&'ctx str> {
        self.raw.find_module_in_descendants()
    }
}

impl std::fmt::Debug for Outlined<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Outlined")
            .field("kind", &self.kind())
            .field("index", &self.index())
            .field("module", &self.module())
            .finish()
    }
}

impl std::fmt::Display for Outlined<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.raw)
    }
}

/// The kind of outlined operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutlinedKind {
    /// Outlined Objective-C bridged method.
    BridgedMethod,
    /// Outlined variable (stack allocation).
    Variable,
    /// Outlined copy operation.
    Copy,
    /// Outlined consume operation.
    Consume,
    /// Outlined retain operation.
    Retain,
    /// Outlined release operation.
    Release,
    /// Outlined initialize with take.
    InitializeWithTake,
    /// Outlined initialize with copy.
    InitializeWithCopy,
    /// Outlined assign with take.
    AssignWithTake,
    /// Outlined assign with copy.
    AssignWithCopy,
    /// Outlined destroy operation.
    Destroy,
    /// Outlined initialize with copy (no value witness).
    InitializeWithCopyNoValueWitness,
    /// Outlined initialize with take (no value witness).
    InitializeWithTakeNoValueWitness,
    /// Outlined assign with copy (no value witness).
    AssignWithCopyNoValueWitness,
    /// Outlined assign with take (no value witness).
    AssignWithTakeNoValueWitness,
    /// Outlined destroy (no value witness).
    DestroyNoValueWitness,
    /// Outlined read-only object.
    ReadOnlyObject,
    /// Other outlined operation.
    Other,
}

impl OutlinedKind {
    /// Get a human-readable name for this outlined kind.
    pub fn name(&self) -> &'static str {
        match self {
            OutlinedKind::BridgedMethod => "outlined bridged method",
            OutlinedKind::Variable => "outlined variable",
            OutlinedKind::Copy => "outlined copy",
            OutlinedKind::Consume => "outlined consume",
            OutlinedKind::Retain => "outlined retain",
            OutlinedKind::Release => "outlined release",
            OutlinedKind::InitializeWithTake => "outlined initializeWithTake",
            OutlinedKind::InitializeWithCopy => "outlined initializeWithCopy",
            OutlinedKind::AssignWithTake => "outlined assignWithTake",
            OutlinedKind::AssignWithCopy => "outlined assignWithCopy",
            OutlinedKind::Destroy => "outlined destroy",
            OutlinedKind::InitializeWithCopyNoValueWitness => {
                "outlined initializeWithCopy (no value witness)"
            }
            OutlinedKind::InitializeWithTakeNoValueWitness => {
                "outlined initializeWithTake (no value witness)"
            }
            OutlinedKind::AssignWithCopyNoValueWitness => {
                "outlined assignWithCopy (no value witness)"
            }
            OutlinedKind::AssignWithTakeNoValueWitness => {
                "outlined assignWithTake (no value witness)"
            }
            OutlinedKind::DestroyNoValueWitness => "outlined destroy (no value witness)",
            OutlinedKind::ReadOnlyObject => "outlined read-only object",
            OutlinedKind::Other => "outlined operation",
        }
    }
}
