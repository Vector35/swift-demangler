//! Macro symbol representation.
//!
//! These symbols represent Swift macros and their expansions.

use crate::helpers::NodeExt;
use crate::raw::{Node, NodeKind};

/// A Swift macro symbol.
#[derive(Clone, Copy)]
pub struct MacroSymbol<'ctx> {
    raw: Node<'ctx>,
}

impl<'ctx> MacroSymbol<'ctx> {
    /// Create a MacroSymbol from a raw node.
    pub fn new(raw: Node<'ctx>) -> Self {
        Self { raw }
    }

    /// Get the underlying raw node.
    pub fn raw(&self) -> Node<'ctx> {
        self.raw
    }

    /// Get the kind of macro symbol.
    pub fn kind(&self) -> MacroSymbolKind {
        match self.raw.kind() {
            NodeKind::Macro => MacroSymbolKind::Definition,
            NodeKind::FreestandingMacroExpansion => MacroSymbolKind::FreestandingExpansion,
            NodeKind::MacroExpansionUniqueName => MacroSymbolKind::ExpansionUniqueName,
            NodeKind::AccessorAttachedMacroExpansion
            | NodeKind::BodyAttachedMacroExpansion
            | NodeKind::ConformanceAttachedMacroExpansion
            | NodeKind::ExtensionAttachedMacroExpansion
            | NodeKind::MemberAttachedMacroExpansion
            | NodeKind::MemberAttributeAttachedMacroExpansion
            | NodeKind::PeerAttachedMacroExpansion
            | NodeKind::PreambleAttachedMacroExpansion => MacroSymbolKind::AttachedExpansion,
            _ => MacroSymbolKind::Other,
        }
    }

    /// Get the macro name.
    pub fn name(&self) -> Option<&'ctx str> {
        self.raw
            .child_of_kind(NodeKind::Identifier)
            .and_then(|c| c.text())
    }

    /// Get the discriminator/index of this expansion (0-indexed).
    pub fn discriminator(&self) -> Option<u64> {
        for child in self.raw.children() {
            if child.kind() == NodeKind::Number || child.kind() == NodeKind::Index {
                return child.index();
            }
        }
        None
    }

    /// Get the expansion number for display (1-indexed).
    ///
    /// This matches the `#N` shown in the demangled output.
    pub fn expansion_number(&self) -> Option<u64> {
        self.discriminator().map(|d| d + 1)
    }

    /// Get the module containing this macro.
    pub fn module(&self) -> Option<&'ctx str> {
        // First check for Module node directly
        for node in self.raw.descendants() {
            if node.kind() == NodeKind::Module {
                return node.text();
            }
        }
        // For macro expansions, the module is the first Identifier in MacroExpansionLoc
        if let Some(loc) = self.expansion_loc() {
            let mut identifiers = loc.children().filter(|c| c.kind() == NodeKind::Identifier);
            return identifiers.next().and_then(|id| id.text());
        }
        None
    }

    /// Get the MacroExpansionLoc node if present.
    fn expansion_loc(&self) -> Option<Node<'ctx>> {
        self.raw.child_of_kind(NodeKind::MacroExpansionLoc)
    }

    /// Get the source file name for this macro expansion.
    pub fn file(&self) -> Option<&'ctx str> {
        if let Some(loc) = self.expansion_loc() {
            // File is the second Identifier in MacroExpansionLoc
            let mut identifiers = loc.children().filter(|c| c.kind() == NodeKind::Identifier);
            identifiers.next(); // skip module
            return identifiers.next().and_then(|id| id.text());
        }
        None
    }

    /// Get the source line number for this macro expansion.
    pub fn line(&self) -> Option<u64> {
        if let Some(loc) = self.expansion_loc() {
            // Line is the first Index in MacroExpansionLoc
            let mut indexes = loc.children().filter(|c| c.kind() == NodeKind::Index);
            return indexes.next().and_then(|n| n.index());
        }
        None
    }

    /// Get the source column number for this macro expansion.
    pub fn column(&self) -> Option<u64> {
        if let Some(loc) = self.expansion_loc() {
            // Column is the second Index in MacroExpansionLoc
            let mut indexes = loc.children().filter(|c| c.kind() == NodeKind::Index);
            indexes.next(); // skip line
            return indexes.next().and_then(|n| n.index());
        }
        None
    }
}

impl std::fmt::Debug for MacroSymbol<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = f.debug_struct("MacroSymbol");
        s.field("kind", &self.kind())
            .field("name", &self.name())
            .field("expansion_number", &self.expansion_number())
            .field("module", &self.module());
        if let Some(file) = self.file() {
            s.field("file", &file);
        }
        if let Some(line) = self.line() {
            s.field("line", &line);
        }
        if let Some(column) = self.column() {
            s.field("column", &column);
        }
        s.finish()
    }
}

impl std::fmt::Display for MacroSymbol<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.raw)
    }
}

/// The kind of macro symbol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MacroSymbolKind {
    /// A macro definition.
    Definition,
    /// A freestanding macro expansion.
    FreestandingExpansion,
    /// A unique name generated by macro expansion.
    ExpansionUniqueName,
    /// An attached macro expansion.
    AttachedExpansion,
    /// Other macro-related symbol.
    Other,
}

impl MacroSymbolKind {
    /// Get a human-readable name for this macro symbol kind.
    pub fn name(&self) -> &'static str {
        match self {
            MacroSymbolKind::Definition => "macro definition",
            MacroSymbolKind::FreestandingExpansion => "freestanding macro expansion",
            MacroSymbolKind::ExpansionUniqueName => "macro expansion unique name",
            MacroSymbolKind::AttachedExpansion => "attached macro expansion",
            MacroSymbolKind::Other => "macro symbol",
        }
    }
}
