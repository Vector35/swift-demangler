//! Symbol context representation.
//!
//! This module provides types for representing the context (location) of a Swift symbol,
//! such as the module, type, and extension path.

use crate::raw::{Node, NodeKind};

/// The context (location) of a Swift symbol.
///
/// A context represents where a symbol is defined, including module, type, and extension information.
#[derive(Clone, Copy)]
pub struct SymbolContext<'ctx> {
    raw: Node<'ctx>,
}

impl<'ctx> SymbolContext<'ctx> {
    /// Create a SymbolContext from a raw node.
    pub fn new(raw: Node<'ctx>) -> Self {
        Self { raw }
    }

    /// Get the underlying raw node.
    pub fn raw(&self) -> Node<'ctx> {
        self.raw
    }

    /// Get the module name if this context is in a module.
    pub fn module(&self) -> Option<&'ctx str> {
        Self::find_module_in_context(self.raw)
    }

    fn find_module_in_context(node: Node<'ctx>) -> Option<&'ctx str> {
        // First check direct children
        for child in node.children() {
            if child.kind() == NodeKind::Module {
                return child.text();
            }
        }
        // Then check the context chain
        for child in node.children() {
            match child.kind() {
                NodeKind::Class
                | NodeKind::Structure
                | NodeKind::Enum
                | NodeKind::Protocol
                | NodeKind::Extension
                | NodeKind::TypeAlias
                | NodeKind::OtherNominalType => {
                    if let Some(module) = Self::find_module_in_context(child) {
                        return Some(module);
                    }
                }
                _ => {}
            }
        }
        None
    }

    /// Get the type name if this context is within a type.
    pub fn type_name(&self) -> Option<&'ctx str> {
        self.find_type_name_in_context(self.raw)
    }

    fn find_type_name_in_context(&self, node: Node<'ctx>) -> Option<&'ctx str> {
        for child in node.children() {
            match child.kind() {
                NodeKind::Class
                | NodeKind::Structure
                | NodeKind::Enum
                | NodeKind::Protocol
                | NodeKind::TypeAlias
                | NodeKind::OtherNominalType => {
                    return self.extract_identifier(child);
                }
                NodeKind::Extension => {
                    // Extension's type is its first child
                    if let Some(inner) = child.child(0) {
                        return self
                            .find_type_name_in_context(inner)
                            .or_else(|| self.extract_identifier(inner));
                    }
                }
                _ => {}
            }
        }
        None
    }

    fn extract_identifier(&self, node: Node<'ctx>) -> Option<&'ctx str> {
        for child in node.children() {
            if child.kind() == NodeKind::Identifier {
                return child.text();
            }
        }
        node.text()
    }

    /// Get the full path as a string (e.g., "ModuleName.TypeName").
    pub fn full_path(&self) -> String {
        let components: Vec<String> = self.components().map(|c| c.name().to_string()).collect();
        components.join(".")
    }

    /// Check if this context is an extension.
    pub fn is_extension(&self) -> bool {
        self.raw.kind() == NodeKind::Extension
            || self.raw.children().any(|c| c.kind() == NodeKind::Extension)
    }

    /// Iterate over the context components from outermost (module) to innermost.
    pub fn components(&self) -> impl Iterator<Item = ContextComponent<'ctx>> + use<'ctx> {
        let mut components = Vec::new();
        self.collect_components(self.raw, &mut components);
        components.into_iter()
    }

    fn collect_components(&self, node: Node<'ctx>, components: &mut Vec<ContextComponent<'ctx>>) {
        for child in node.children() {
            match child.kind() {
                NodeKind::Module => {
                    if let Some(name) = child.text() {
                        components.push(ContextComponent::Module(name));
                    }
                }
                NodeKind::Class => {
                    // First, find and add the module from inside the class
                    self.collect_module_from_type(child, components);
                    // Then add the class itself
                    if let Some(name) = self.extract_identifier(child) {
                        components.push(ContextComponent::Class { name, raw: child });
                    }
                }
                NodeKind::Structure => {
                    self.collect_module_from_type(child, components);
                    if let Some(name) = self.extract_identifier(child) {
                        components.push(ContextComponent::Struct { name, raw: child });
                    }
                }
                NodeKind::Enum => {
                    self.collect_module_from_type(child, components);
                    if let Some(name) = self.extract_identifier(child) {
                        components.push(ContextComponent::Enum { name, raw: child });
                    }
                }
                NodeKind::Protocol => {
                    self.collect_module_from_type(child, components);
                    if let Some(name) = self.extract_identifier(child) {
                        components.push(ContextComponent::Protocol { name, raw: child });
                    }
                }
                NodeKind::Extension => {
                    // Extension wraps the extended type
                    if let Some(extended_type) = child.child(0) {
                        let base = self.context_component_from_type(extended_type);
                        components.push(ContextComponent::Extension {
                            base: Box::new(base),
                            raw: child,
                        });
                    }
                }
                NodeKind::TypeAlias => {
                    self.collect_module_from_type(child, components);
                    if let Some(name) = self.extract_identifier(child) {
                        components.push(ContextComponent::TypeAlias { name, raw: child });
                    }
                }
                _ => {}
            }
        }
    }

    fn collect_module_from_type(
        &self,
        type_node: Node<'ctx>,
        components: &mut Vec<ContextComponent<'ctx>>,
    ) {
        for child in type_node.children() {
            if child.kind() == NodeKind::Module
                && let Some(name) = child.text()
            {
                components.push(ContextComponent::Module(name));
                return;
            }
        }
    }

    fn context_component_from_type(&self, node: Node<'ctx>) -> ContextComponent<'ctx> {
        match node.kind() {
            NodeKind::Class => ContextComponent::Class {
                name: self.extract_identifier(node).unwrap_or(""),
                raw: node,
            },
            NodeKind::Structure => ContextComponent::Struct {
                name: self.extract_identifier(node).unwrap_or(""),
                raw: node,
            },
            NodeKind::Enum => ContextComponent::Enum {
                name: self.extract_identifier(node).unwrap_or(""),
                raw: node,
            },
            NodeKind::Protocol => ContextComponent::Protocol {
                name: self.extract_identifier(node).unwrap_or(""),
                raw: node,
            },
            NodeKind::TypeAlias => ContextComponent::TypeAlias {
                name: self.extract_identifier(node).unwrap_or(""),
                raw: node,
            },
            NodeKind::Module => ContextComponent::Module(node.text().unwrap_or("")),
            _ => ContextComponent::Other(node),
        }
    }
}

impl std::fmt::Debug for SymbolContext<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SymbolContext")
            .field("module", &self.module())
            .field("type_name", &self.type_name())
            .field("full_path", &self.full_path())
            .field("is_extension", &self.is_extension())
            .finish()
    }
}

impl std::fmt::Display for SymbolContext<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.full_path())
    }
}

/// A component in a symbol's context path.
#[derive(Debug)]
pub enum ContextComponent<'ctx> {
    /// A module.
    Module(&'ctx str),
    /// A class.
    Class { name: &'ctx str, raw: Node<'ctx> },
    /// A struct.
    Struct { name: &'ctx str, raw: Node<'ctx> },
    /// An enum.
    Enum { name: &'ctx str, raw: Node<'ctx> },
    /// A protocol.
    Protocol { name: &'ctx str, raw: Node<'ctx> },
    /// An extension of another type.
    Extension {
        base: Box<ContextComponent<'ctx>>,
        raw: Node<'ctx>,
    },
    /// A type alias.
    TypeAlias { name: &'ctx str, raw: Node<'ctx> },
    /// Other context component.
    Other(Node<'ctx>),
}

impl<'ctx> ContextComponent<'ctx> {
    /// Get the name of this context component.
    pub fn name(&self) -> &'ctx str {
        match self {
            ContextComponent::Module(name) => name,
            ContextComponent::Class { name, .. } => name,
            ContextComponent::Struct { name, .. } => name,
            ContextComponent::Enum { name, .. } => name,
            ContextComponent::Protocol { name, .. } => name,
            ContextComponent::Extension { base, .. } => base.name(),
            ContextComponent::TypeAlias { name, .. } => name,
            ContextComponent::Other(_) => "",
        }
    }

    /// Get the raw node for this component, if available.
    pub fn raw(&self) -> Option<Node<'ctx>> {
        match self {
            ContextComponent::Module(_) => None,
            ContextComponent::Class { raw, .. } => Some(*raw),
            ContextComponent::Struct { raw, .. } => Some(*raw),
            ContextComponent::Enum { raw, .. } => Some(*raw),
            ContextComponent::Protocol { raw, .. } => Some(*raw),
            ContextComponent::Extension { raw, .. } => Some(*raw),
            ContextComponent::TypeAlias { raw, .. } => Some(*raw),
            ContextComponent::Other(raw) => Some(*raw),
        }
    }

    /// Check if this component is a type (class, struct, enum, protocol).
    pub fn is_type(&self) -> bool {
        matches!(
            self,
            ContextComponent::Class { .. }
                | ContextComponent::Struct { .. }
                | ContextComponent::Enum { .. }
                | ContextComponent::Protocol { .. }
                | ContextComponent::TypeAlias { .. }
        )
    }

    /// Check if this component is an extension.
    pub fn is_extension(&self) -> bool {
        matches!(self, ContextComponent::Extension { .. })
    }
}

/// Extract the context from a symbol node.
///
/// This function navigates from a symbol node (Function, Getter, etc.)
/// to find its containing context.
pub fn extract_context<'ctx>(symbol_node: Node<'ctx>) -> SymbolContext<'ctx> {
    // The symbol node itself contains the context as children
    // For a function: Function -> [Module, Identifier, Type, ...]
    // For a method: Function -> [Class/Struct/etc -> [Module, Identifier], Identifier, Type, ...]
    SymbolContext::new(symbol_node)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::raw::Context;

    #[test]
    fn test_simple_function_context() {
        let ctx = Context::new();
        // main.hello() async throws -> Swift.String
        let root = Node::parse(&ctx, "$s4main5helloSSyYaKF").unwrap();
        let func = root.child(0).unwrap();
        let context = extract_context(func);

        assert_eq!(context.module(), Some("main"));
    }

    #[test]
    fn test_method_context() {
        let ctx = Context::new();
        // foo.bar.bas(zim: foo.zim) -> ()
        let root = Node::parse(&ctx, "_TFC3foo3bar3basfT3zimCS_3zim_T_").unwrap();
        let func = root.child(0).unwrap();
        let context = extract_context(func);

        assert_eq!(context.module(), Some("foo"));
        assert_eq!(context.type_name(), Some("bar"));
    }

    #[test]
    fn test_context_full_path() {
        let ctx = Context::new();
        // foo.bar.bas(zim: foo.zim) -> () - a method in class bar
        let root = Node::parse(&ctx, "_TFC3foo3bar3basfT3zimCS_3zim_T_").unwrap();
        let func = root.child(0).unwrap();
        let context = extract_context(func);

        // full_path should contain the module and type
        let path = context.full_path();
        assert!(
            path.contains("foo"),
            "path should contain module 'foo': {path}"
        );
        assert!(
            path.contains("bar"),
            "path should contain type 'bar': {path}"
        );
    }

    #[test]
    fn test_context_components() {
        let ctx = Context::new();
        let root = Node::parse(&ctx, "_TFC3foo3bar3basfT3zimCS_3zim_T_").unwrap();
        let func = root.child(0).unwrap();
        let context = extract_context(func);

        let components: Vec<_> = context.components().collect();

        // Should have module and class components
        assert!(!components.is_empty(), "should have at least one component");

        // Verify we can find the module
        let has_module = components.iter().any(|c| c.name() == "foo");
        assert!(
            has_module,
            "should have module 'foo' in components: {:?}",
            components.iter().map(|c| c.name()).collect::<Vec<_>>()
        );
    }
}
