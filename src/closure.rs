//! Closure symbol representation.
//!
//! This module provides types for representing Swift closures.

use crate::context::{SymbolContext, extract_context};
use crate::function::Function;
use crate::helpers::{HasExtensionContext, HasFunctionSignature, HasGenericSignature, NodeExt};
use crate::raw::{Node, NodeKind};
use crate::types::{FunctionType, GenericSignature};

/// A Swift closure symbol.
#[derive(Clone, Copy)]
pub struct Closure<'ctx> {
    raw: Node<'ctx>,
}

impl<'ctx> Closure<'ctx> {
    /// Create a Closure from a raw node.
    pub fn new(raw: Node<'ctx>) -> Self {
        Self { raw }
    }

    /// Get the underlying raw node.
    pub fn raw(&self) -> Node<'ctx> {
        self.raw
    }

    /// Get the parent context where this closure is defined.
    pub fn parent_context(&self) -> SymbolContext<'ctx> {
        extract_context(self.raw)
    }

    /// Get the closure index (for distinguishing multiple closures in the same context).
    pub fn index(&self) -> Option<u64> {
        self.raw.index()
    }

    /// Get the kind of closure.
    pub fn kind(&self) -> ClosureKind {
        match self.raw.kind() {
            NodeKind::ExplicitClosure => ClosureKind::Explicit,
            NodeKind::ImplicitClosure => ClosureKind::Implicit,
            _ => ClosureKind::Explicit,
        }
    }

    /// Get the module containing the parent function.
    pub fn module(&self) -> Option<&'ctx str> {
        // Look for Module in parent function context
        for child in self.raw.children() {
            if child.kind() == NodeKind::Module {
                return child.text();
            }
            if child.kind() == NodeKind::Function {
                // Look at direct children of Function, not descendants,
                // to avoid finding Module nodes inside parameter types
                for inner in child.children() {
                    if inner.kind() == NodeKind::Module {
                        return inner.text();
                    }
                }
            }
        }
        // Fall back to find_module helper which handles type contexts properly
        self.raw.find_module()
    }

    /// Get the parent function if available.
    pub fn parent_function(&self) -> Option<Function<'ctx>> {
        self.raw
            .child_of_kind(NodeKind::Function)
            .map(Function::new)
    }

    /// Get the discriminator (unique identifier within parent).
    pub fn discriminator(&self) -> Option<u64> {
        // Check for index on the closure node itself
        if let Some(idx) = self.raw.index() {
            return Some(idx);
        }
        // Look for Number child
        for child in self.raw.children() {
            if child.kind() == NodeKind::Number {
                return child.index();
            }
        }
        None
    }
}

impl<'ctx> HasGenericSignature<'ctx> for Closure<'ctx> {
    fn generic_signature(&self) -> Option<GenericSignature<'ctx>> {
        // First try the standard search
        if let Some(sig) = self.raw.find_generic_signature() {
            return Some(sig);
        }
        // Check for constrained extension in parent function context
        for child in self.raw.children() {
            if child.kind() == NodeKind::Function
                && let Some(sig) = child.find_generic_signature()
            {
                return Some(sig);
            }
        }
        None
    }
}

impl<'ctx> HasFunctionSignature<'ctx> for Closure<'ctx> {
    fn signature(&self) -> Option<FunctionType<'ctx>> {
        self.raw.find_function_type()
    }
}

impl<'ctx> HasExtensionContext<'ctx> for Closure<'ctx> {
    fn raw(&self) -> Node<'ctx> {
        self.raw
    }
}

impl std::fmt::Debug for Closure<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = f.debug_struct("Closure");
        s.field("kind", &self.kind())
            .field("index", &self.index())
            .field("discriminator", &self.discriminator())
            .field("module", &self.module())
            .field("parent_function", &self.parent_function())
            .field("is_async", &self.is_async())
            .field("is_throwing", &self.is_throwing())
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

impl std::fmt::Display for Closure<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.raw)
    }
}

/// The kind of closure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClosureKind {
    /// An explicit closure (written with `{ ... }`).
    Explicit,
    /// An implicit closure (compiler-generated, e.g., for lazy properties).
    Implicit,
}

impl ClosureKind {
    /// Get a human-readable name for this closure kind.
    pub fn name(&self) -> &'static str {
        match self {
            ClosureKind::Explicit => "explicit",
            ClosureKind::Implicit => "implicit",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_closure_kind_name() {
        assert_eq!(ClosureKind::Explicit.name(), "explicit");
        assert_eq!(ClosureKind::Implicit.name(), "implicit");
    }

    #[test]
    fn test_closure_kind_equality() {
        assert_eq!(ClosureKind::Explicit, ClosureKind::Explicit);
        assert_ne!(ClosureKind::Explicit, ClosureKind::Implicit);
    }
}
