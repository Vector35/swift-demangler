//! Type representation for Swift symbols.
//!
//! This module provides semantic types for representing Swift types
//! extracted from demangled symbols.

use crate::helpers::NodeExt;
use crate::raw::{Node, NodeKind};

/// A reference to a Swift type.
///
/// This wraps a raw node and provides methods to extract type information.
/// The type information is parsed lazily when accessed.
#[derive(Clone, Copy)]
pub struct TypeRef<'ctx> {
    raw: Node<'ctx>,
}

impl<'ctx> TypeRef<'ctx> {
    /// Create a TypeRef from a raw node.
    pub fn new(raw: Node<'ctx>) -> Self {
        Self { raw }
    }

    /// Get the underlying raw node.
    pub fn raw(&self) -> Node<'ctx> {
        self.raw
    }

    /// Get the kind of this type.
    pub fn kind(&self) -> TypeKind<'ctx> {
        self.classify()
    }

    /// Get a display string for this type.
    pub fn display(&self) -> String {
        self.raw.to_string()
    }

    /// Get generic arguments if this is a generic type.
    ///
    /// Returns an empty vector if this is not a generic type.
    pub fn generic_args(&self) -> Vec<TypeRef<'ctx>> {
        if let TypeKind::Named(named) = self.kind() {
            named.generic_args()
        } else {
            Vec::new()
        }
    }

    /// Check if this is a generic type.
    pub fn is_generic(&self) -> bool {
        if let TypeKind::Named(named) = self.kind() {
            named.is_generic()
        } else {
            false
        }
    }

    /// Helper for type wrappers that take child(0) and wrap in a TypeKind variant.
    fn wrap_child<F>(&self, wrapper: F) -> TypeKind<'ctx>
    where
        F: FnOnce(Box<TypeRef<'ctx>>) -> TypeKind<'ctx>,
    {
        self.raw
            .child(0)
            .map(|inner| wrapper(Box::new(TypeRef::new(inner))))
            .unwrap_or(TypeKind::Other(self.raw))
    }

    /// Helper for types that need to unwrap two layers (child(0).child(0)).
    fn wrap_nested_child<F>(&self, wrapper: F) -> TypeKind<'ctx>
    where
        F: FnOnce(Box<TypeRef<'ctx>>) -> TypeKind<'ctx>,
    {
        self.raw
            .child(0)
            .and_then(|t| t.child(0))
            .map(|inner| wrapper(Box::new(TypeRef::new(inner))))
            .unwrap_or(TypeKind::Other(self.raw))
    }

    fn classify(&self) -> TypeKind<'ctx> {
        match self.raw.kind() {
            // Named types (classes, structs, enums, protocols, type aliases)
            NodeKind::Class
            | NodeKind::Structure
            | NodeKind::Enum
            | NodeKind::Protocol
            | NodeKind::TypeAlias
            | NodeKind::OtherNominalType => TypeKind::Named(NamedType { raw: self.raw }),

            // Bound generic types
            NodeKind::BoundGenericClass
            | NodeKind::BoundGenericStructure
            | NodeKind::BoundGenericEnum
            | NodeKind::BoundGenericProtocol
            | NodeKind::BoundGenericTypeAlias
            | NodeKind::BoundGenericOtherNominalType => {
                TypeKind::Named(NamedType { raw: self.raw })
            }

            // Function types
            NodeKind::FunctionType
            | NodeKind::NoEscapeFunctionType
            | NodeKind::CFunctionPointer
            | NodeKind::ThinFunctionType
            | NodeKind::AutoClosureType
            | NodeKind::EscapingAutoClosureType
            | NodeKind::ObjCBlock
            | NodeKind::EscapingObjCBlock
            | NodeKind::ConcurrentFunctionType
            | NodeKind::GlobalActorFunctionType
            | NodeKind::DifferentiableFunctionType
            | NodeKind::IsolatedAnyFunctionType
            | NodeKind::NonIsolatedCallerFunctionType
            | NodeKind::SendingResultFunctionType
            | NodeKind::UncurriedFunctionType => TypeKind::Function(FunctionType { raw: self.raw }),

            // SIL implementation function type (with detailed conventions)
            NodeKind::ImplFunctionType => TypeKind::ImplFunction(ImplFunctionType::new(self.raw)),

            // Tuple
            NodeKind::Tuple => {
                let elements = self
                    .raw
                    .children()
                    .filter(|c| c.kind() == NodeKind::TupleElement)
                    .map(TupleElement::new)
                    .collect();
                TypeKind::Tuple(elements)
            }

            // Optional types (sugared)
            NodeKind::SugaredOptional => self.wrap_child(TypeKind::Optional),

            // Array types (sugared)
            NodeKind::SugaredArray => self.wrap_child(TypeKind::Array),

            // Dictionary types (sugared)
            NodeKind::SugaredDictionary => {
                if let (Some(key), Some(value)) = (self.raw.child(0), self.raw.child(1)) {
                    TypeKind::Dictionary {
                        key: Box::new(TypeRef::new(key)),
                        value: Box::new(TypeRef::new(value)),
                    }
                } else {
                    TypeKind::Other(self.raw)
                }
            }

            // Generic type parameters
            NodeKind::DependentGenericParamType => {
                let depth = self.raw.child(0).and_then(|c| c.index()).unwrap_or(0);
                let index = self.raw.child(1).and_then(|c| c.index()).unwrap_or(0);
                TypeKind::GenericParam { depth, index }
            }

            // Metatype
            NodeKind::Metatype | NodeKind::ExistentialMetatype => {
                if let Some(inner) = self.raw.child(0) {
                    // The child might be a MetatypeRepresentation or the actual type
                    let type_node = if inner.kind() == NodeKind::MetatypeRepresentation {
                        self.raw.child(1)
                    } else {
                        Some(inner)
                    };
                    if let Some(t) = type_node {
                        TypeKind::Metatype(Box::new(TypeRef::new(t)))
                    } else {
                        TypeKind::Other(self.raw)
                    }
                } else {
                    TypeKind::Other(self.raw)
                }
            }

            // Existential (protocols)
            NodeKind::ProtocolList
            | NodeKind::ProtocolListWithClass
            | NodeKind::ProtocolListWithAnyObject => {
                let protocols: Vec<_> = self
                    .raw
                    .descendants()
                    .filter(|n| n.kind() == NodeKind::Protocol || n.kind() == NodeKind::Type)
                    .map(TypeRef::new)
                    .collect();
                if protocols.is_empty() {
                    // Empty protocol list represents `Any`
                    TypeKind::Any
                } else {
                    TypeKind::Existential(protocols)
                }
            }

            // Builtin types
            NodeKind::BuiltinTypeName => {
                let name = self.raw.text().unwrap_or("");
                TypeKind::Builtin(name)
            }

            // Builtin fixed array
            NodeKind::BuiltinFixedArray => {
                // Structure: BuiltinFixedArray -> [Type -> size, Type -> element]
                let mut children = self.raw.children();
                let size = children
                    .next()
                    .and_then(|t| t.child(0))
                    .and_then(|n| n.index())
                    .map(|i| i as i64);
                let element = children
                    .next()
                    .and_then(|t| t.child(0))
                    .map(|n| Box::new(TypeRef::new(n)));
                TypeKind::BuiltinFixedArray { size, element }
            }

            // InOut types
            NodeKind::InOut => self.wrap_child(TypeKind::InOut),

            // Ownership modifiers
            NodeKind::Shared => self.wrap_child(TypeKind::Shared),
            NodeKind::Owned => self.wrap_child(TypeKind::Owned),
            NodeKind::Weak => self.wrap_nested_child(TypeKind::Weak),
            NodeKind::Unowned => self.wrap_nested_child(TypeKind::Unowned),
            NodeKind::Sending => self.wrap_child(TypeKind::Sending),
            NodeKind::Isolated => self.wrap_child(TypeKind::Isolated),

            // NoDerivative type wrapper (autodiff)
            NodeKind::NoDerivative => self.wrap_child(TypeKind::NoDerivative),

            // Variadic generic pack
            NodeKind::Pack => {
                let elements: Vec<_> = self
                    .raw
                    .children()
                    .filter(|c| c.kind() == NodeKind::Type)
                    .filter_map(|t| t.child(0))
                    .map(TypeRef::new)
                    .collect();
                TypeKind::Pack(elements)
            }

            // Value generics (integer literals as generic arguments)
            NodeKind::Integer => {
                let value = self.raw.index().map(|i| i as i64).unwrap_or(0);
                TypeKind::ValueGeneric(value)
            }

            // Compile-time literal type (macro-related)
            NodeKind::CompileTimeLiteral => self.wrap_child(TypeKind::CompileTimeLiteral),

            // Error type (invalid/failed demangling)
            NodeKind::ErrorType => TypeKind::Error,

            // Type wrapper node - unwrap and classify the inner type
            NodeKind::Type => {
                if let Some(inner) = self.raw.child(0) {
                    TypeRef::new(inner).classify()
                } else {
                    TypeKind::Other(self.raw)
                }
            }

            // Dynamic self
            NodeKind::DynamicSelf => self.wrap_child(TypeKind::DynamicSelf),

            // Constrained existential
            NodeKind::ConstrainedExistential => self.wrap_child(TypeKind::ConstrainedExistential),

            // Module - typically part of a qualified name
            NodeKind::Module => TypeKind::Named(NamedType { raw: self.raw }),

            // Dependent generic type - preserve generic signature if present
            NodeKind::DependentGenericType => {
                // Structure: DependentGenericType -> [DependentGenericSignature, Type -> inner]
                let signature = self
                    .raw
                    .child_of_kind(NodeKind::DependentGenericSignature)
                    .map(GenericSignature::new);

                let inner = self
                    .raw
                    .child_of_kind(NodeKind::Type)
                    .and_then(|t| t.child(0));

                match (signature, inner) {
                    (Some(sig), Some(inner_node)) => TypeKind::Generic {
                        signature: sig,
                        inner: Box::new(TypeRef::new(inner_node)),
                    },
                    (None, Some(inner_node)) => TypeRef::new(inner_node).kind(),
                    _ => TypeKind::Other(self.raw),
                }
            }

            // Associated type (T.AssociatedType)
            NodeKind::DependentMemberType => {
                // Structure: DependentMemberType -> [Type (base), DependentAssociatedTypeRef -> Identifier]
                let base = self
                    .raw
                    .child_of_kind(NodeKind::Type)
                    .and_then(|t| t.child(0))
                    .map(|inner| Box::new(TypeRef::new(inner)));
                let name = self
                    .raw
                    .child_of_kind(NodeKind::DependentAssociatedTypeRef)
                    .and_then(|r| r.child_of_kind(NodeKind::Identifier))
                    .and_then(|id| id.text());
                if let Some(base) = base {
                    TypeKind::AssociatedType { base, name }
                } else {
                    TypeKind::Other(self.raw)
                }
            }

            // Opaque types (some Protocol)
            NodeKind::OpaqueType => {
                // Structure: OpaqueType -> [OpaqueReturnTypeOf -> Function, Index, TypeList]
                let source = self
                    .raw
                    .child_of_kind(NodeKind::OpaqueReturnTypeOf)
                    .map(OpaqueSource::from_opaque_return_type_of);
                let index = self
                    .raw
                    .child_of_kind(NodeKind::Index)
                    .and_then(|i| i.index());
                TypeKind::Opaque { source, index }
            }

            NodeKind::OpaqueReturnType => {
                // Structure: OpaqueReturnType -> [OpaqueReturnTypeParent]
                // The parent contains the mangled name of the defining function
                let source = self
                    .raw
                    .child_of_kind(NodeKind::OpaqueReturnTypeParent)
                    .map(OpaqueSource::from_opaque_return_type_parent);
                TypeKind::Opaque {
                    source,
                    index: None,
                }
            }

            // SIL box types (captured values in closures)
            NodeKind::SILBoxTypeWithLayout | NodeKind::SILBoxType => {
                // Structure: SILBoxTypeWithLayout -> [SILBoxLayout, DependentGenericSignature?, TypeList?]
                // SILBoxLayout -> [SILBoxMutableField | SILBoxImmutableField]
                let mut fields = Vec::new();
                let mut substitutions = Vec::new();

                for child in self.raw.children() {
                    match child.kind() {
                        NodeKind::SILBoxLayout => {
                            for field_node in child.children() {
                                let is_mutable = field_node.kind() == NodeKind::SILBoxMutableField;
                                if let Some(type_node) = field_node.child(0) {
                                    let inner = if type_node.kind() == NodeKind::Type {
                                        type_node.child(0).unwrap_or(type_node)
                                    } else {
                                        type_node
                                    };
                                    fields.push(SILBoxField {
                                        type_ref: TypeRef::new(inner),
                                        is_mutable,
                                    });
                                }
                            }
                        }
                        NodeKind::TypeList => {
                            for type_node in child.children() {
                                if let Some(inner) = type_node.unwrap_if_kind(NodeKind::Type) {
                                    substitutions.push(TypeRef::new(inner));
                                }
                            }
                        }
                        _ => {}
                    }
                }

                TypeKind::SILBox {
                    fields,
                    substitutions,
                }
            }

            // Fallback
            _ => TypeKind::Other(self.raw),
        }
    }
}

impl std::fmt::Debug for TypeRef<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Just delegate to the kind's debug representation
        self.kind().fmt(f)
    }
}

impl std::fmt::Display for TypeRef<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display())
    }
}

/// The kind of a Swift type.
#[derive(Debug)]
pub enum TypeKind<'ctx> {
    /// A named type (class, struct, enum, protocol, type alias).
    Named(NamedType<'ctx>),
    /// A function type.
    Function(FunctionType<'ctx>),
    /// A SIL implementation function type (with detailed conventions).
    ImplFunction(ImplFunctionType<'ctx>),
    /// A tuple type.
    Tuple(Vec<TupleElement<'ctx>>),
    /// An optional type (`T?`).
    Optional(Box<TypeRef<'ctx>>),
    /// An array type (`[T]`).
    Array(Box<TypeRef<'ctx>>),
    /// A dictionary type (`[K: V]`).
    Dictionary {
        key: Box<TypeRef<'ctx>>,
        value: Box<TypeRef<'ctx>>,
    },
    /// A generic type parameter.
    GenericParam { depth: u64, index: u64 },
    /// A metatype (`T.Type`).
    Metatype(Box<TypeRef<'ctx>>),
    /// An existential type (protocol composition).
    Existential(Vec<TypeRef<'ctx>>),
    /// The `Any` type (empty protocol composition).
    Any,
    /// A builtin type.
    Builtin(&'ctx str),
    /// A builtin fixed array type.
    BuiltinFixedArray {
        size: Option<i64>,
        element: Option<Box<TypeRef<'ctx>>>,
    },
    /// An inout parameter type.
    InOut(Box<TypeRef<'ctx>>),
    /// A shared/borrowed parameter type (`__shared` / `borrowing`).
    Shared(Box<TypeRef<'ctx>>),
    /// An owned/consuming parameter type (`__owned` / `consuming`).
    Owned(Box<TypeRef<'ctx>>),
    /// A weak reference type.
    Weak(Box<TypeRef<'ctx>>),
    /// An unowned reference type.
    Unowned(Box<TypeRef<'ctx>>),
    /// A sending parameter/result type (for region-based isolation).
    Sending(Box<TypeRef<'ctx>>),
    /// An isolated parameter type (actor isolation).
    Isolated(Box<TypeRef<'ctx>>),
    /// A `@noDerivative` type (autodiff).
    NoDerivative(Box<TypeRef<'ctx>>),
    /// A variadic generic pack type.
    Pack(Vec<TypeRef<'ctx>>),
    /// A value generic parameter (integer literal as generic argument).
    ValueGeneric(i64),
    /// A compile-time literal type (macro-related).
    CompileTimeLiteral(Box<TypeRef<'ctx>>),
    /// Dynamic Self type.
    DynamicSelf(Box<TypeRef<'ctx>>),
    /// Constrained existential type.
    ConstrainedExistential(Box<TypeRef<'ctx>>),
    /// An associated type (`T.AssociatedType`).
    AssociatedType {
        base: Box<TypeRef<'ctx>>,
        name: Option<&'ctx str>,
    },
    /// An opaque type (`some Protocol` return type).
    Opaque {
        /// Information about the function that defines this opaque type.
        source: Option<OpaqueSource<'ctx>>,
        /// The index of this opaque type (for multiple `some` returns).
        index: Option<u64>,
    },
    /// A generic type with constraints (e.g., `<A where A: Protocol>(A) -> A.Mince`).
    Generic {
        /// The generic signature containing type parameters and constraints.
        signature: GenericSignature<'ctx>,
        /// The inner type being constrained.
        inner: Box<TypeRef<'ctx>>,
    },
    /// An error type (invalid/failed demangling).
    Error,
    /// A SIL box type (captured values in closures).
    SILBox {
        /// The field types in the box.
        fields: Vec<SILBoxField<'ctx>>,
        /// Substituted types.
        substitutions: Vec<TypeRef<'ctx>>,
    },
    /// Fallback for unhandled type kinds.
    Other(Node<'ctx>),
}

/// A field in a SIL box type.
#[derive(Debug)]
pub struct SILBoxField<'ctx> {
    /// The type of the field.
    pub type_ref: TypeRef<'ctx>,
    /// Whether the field is mutable.
    pub is_mutable: bool,
}

/// Information about the source of an opaque return type.
///
/// This captures the defining function for a `some Protocol` return type.
#[derive(Debug, Clone, Copy)]
pub struct OpaqueSource<'ctx> {
    /// The module containing the defining function.
    pub module: Option<&'ctx str>,
    /// The containing type (struct, class, enum) if this is a method.
    pub containing_type: Option<&'ctx str>,
    /// The name of the defining function.
    pub name: Option<&'ctx str>,
    /// The mangled name of the defining function (fallback for OpaqueReturnType).
    pub mangled_name: Option<&'ctx str>,
}

impl<'ctx> OpaqueSource<'ctx> {
    /// Create an OpaqueSource from an OpaqueReturnTypeOf node.
    fn from_opaque_return_type_of(node: Node<'ctx>) -> Self {
        // OpaqueReturnTypeOf -> Function -> [Structure/Class/Enum/Module, Identifier, ...]
        let func_node = node.child_of_kind(NodeKind::Function);

        let (module, containing_type, name) = if let Some(func) = func_node {
            let module = func
                .descendants()
                .find(|d| d.kind() == NodeKind::Module)
                .and_then(|m| m.text());

            let containing_type = func
                .children()
                .find(|c| {
                    matches!(
                        c.kind(),
                        NodeKind::Structure | NodeKind::Class | NodeKind::Enum | NodeKind::Protocol
                    )
                })
                .and_then(|t| {
                    t.child_of_kind(NodeKind::Identifier)
                        .and_then(|id| id.text())
                });

            let name = func
                .child_of_kind(NodeKind::Identifier)
                .and_then(|id| id.text());

            (module, containing_type, name)
        } else {
            (None, None, None)
        };

        Self {
            module,
            containing_type,
            name,
            mangled_name: None,
        }
    }

    /// Create an OpaqueSource from an OpaqueReturnTypeParent node.
    fn from_opaque_return_type_parent(node: Node<'ctx>) -> Self {
        // OpaqueReturnTypeParent has a text field with the mangled name
        Self {
            module: None,
            containing_type: None,
            name: None,
            mangled_name: node.text(),
        }
    }
}

/// A named Swift type (class, struct, enum, protocol, type alias).
#[derive(Clone, Copy)]
pub struct NamedType<'ctx> {
    raw: Node<'ctx>,
}

impl<'ctx> NamedType<'ctx> {
    /// Get the underlying raw node.
    pub fn raw(&self) -> Node<'ctx> {
        self.raw
    }

    /// Get the simple name of this type (without module qualification).
    pub fn name(&self) -> Option<&'ctx str> {
        // For bound generic types, the name is in the first child
        match self.raw.kind() {
            NodeKind::BoundGenericClass
            | NodeKind::BoundGenericStructure
            | NodeKind::BoundGenericEnum
            | NodeKind::BoundGenericProtocol
            | NodeKind::BoundGenericTypeAlias
            | NodeKind::BoundGenericOtherNominalType => {
                self.raw.child(0).and_then(Self::extract_name)
            }
            _ => Self::extract_name(self.raw),
        }
    }

    fn extract_name(node: Node<'ctx>) -> Option<&'ctx str> {
        // First check if this node has an Identifier child
        for child in node.children() {
            if child.kind() == NodeKind::Identifier {
                return child.text();
            }
        }
        // If this is a Type wrapper, unwrap it and recurse
        if let Some(inner) = node.unwrap_if_kind(NodeKind::Type) {
            return Self::extract_name(inner);
        }
        // Fall back to the node's own text
        node.text()
    }

    /// Get the module containing this type.
    pub fn module(&self) -> Option<&'ctx str> {
        Self::find_module_in_node(self.raw)
    }

    fn find_module_in_node(node: Node<'ctx>) -> Option<&'ctx str> {
        // For bound generic types, the module is in the first child (Type -> actual type)
        let search_node = match node.kind() {
            NodeKind::BoundGenericClass
            | NodeKind::BoundGenericStructure
            | NodeKind::BoundGenericEnum
            | NodeKind::BoundGenericProtocol
            | NodeKind::BoundGenericTypeAlias
            | NodeKind::BoundGenericOtherNominalType => {
                // Get the first child and unwrap Type if needed
                node.child(0).unwrap_or(node)
            }
            NodeKind::Type => {
                // Unwrap Type wrapper
                return node.child(0).and_then(Self::find_module_in_node);
            }
            _ => node,
        };

        for child in search_node.children() {
            if child.kind() == NodeKind::Module {
                return child.text();
            }
            // Recurse into Type wrappers and nominal types
            match child.kind() {
                NodeKind::Type
                | NodeKind::Class
                | NodeKind::Structure
                | NodeKind::Enum
                | NodeKind::Protocol
                | NodeKind::TypeAlias => {
                    if let Some(module) = Self::find_module_in_node(child) {
                        return Some(module);
                    }
                }
                _ => {}
            }
        }
        None
    }

    /// Get the full qualified name of this type.
    pub fn full_name(&self) -> String {
        self.raw.to_string()
    }

    /// Get the generic arguments if this is a bound generic type.
    pub fn generic_args(&self) -> Vec<TypeRef<'ctx>> {
        match self.raw.kind() {
            NodeKind::BoundGenericClass
            | NodeKind::BoundGenericStructure
            | NodeKind::BoundGenericEnum
            | NodeKind::BoundGenericProtocol
            | NodeKind::BoundGenericTypeAlias
            | NodeKind::BoundGenericOtherNominalType => {
                // The generic arguments come after the first child (the base type)
                // They're wrapped in a TypeList node
                self.raw
                    .child(1)
                    .map(|type_list| {
                        type_list
                            .children()
                            .filter(|c| c.kind() == NodeKind::Type)
                            .map(|c| TypeRef::new(c.child(0).unwrap_or(c)))
                            .collect()
                    })
                    .unwrap_or_default()
            }
            _ => Vec::new(),
        }
    }

    /// Check if this is a class type (reference type, always a pointer at the ABI level).
    pub fn is_class(&self) -> bool {
        matches!(
            self.raw.kind(),
            NodeKind::Class | NodeKind::BoundGenericClass
        )
    }

    /// Check if this is a bound generic type.
    pub fn is_generic(&self) -> bool {
        matches!(
            self.raw.kind(),
            NodeKind::BoundGenericClass
                | NodeKind::BoundGenericStructure
                | NodeKind::BoundGenericEnum
                | NodeKind::BoundGenericProtocol
                | NodeKind::BoundGenericTypeAlias
                | NodeKind::BoundGenericOtherNominalType
        )
    }
}

impl std::fmt::Debug for NamedType<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = f.debug_struct("NamedType");
        s.field("module", &self.module());
        s.field("name", &self.name());
        let args = self.generic_args();
        if !args.is_empty() {
            s.field("generic_args", &args);
        }
        s.finish()
    }
}

/// A Swift function type.
#[derive(Clone, Copy)]
pub struct FunctionType<'ctx> {
    raw: Node<'ctx>,
}

impl<'ctx> FunctionType<'ctx> {
    /// Create a FunctionType from a raw node.
    pub fn new(raw: Node<'ctx>) -> Self {
        Self { raw }
    }

    /// Get the underlying raw node.
    pub fn raw(&self) -> Node<'ctx> {
        self.raw
    }

    /// Get the parameters of this function type.
    pub fn parameters(&self) -> Vec<FunctionParam<'ctx>> {
        // Function type structure: FunctionType -> ArgumentTuple -> Type -> Tuple/Type
        // Or for single param: FunctionType -> ArgumentTuple -> Type -> actual_type
        for child in self.raw.children() {
            if child.kind() == NodeKind::ArgumentTuple {
                return self.extract_params_from_argument_tuple(child);
            }
            // ImplFunctionType has ImplParameter children directly
            if child.kind() == NodeKind::ImplParameter {
                return self.extract_impl_params();
            }
        }
        Vec::new()
    }

    fn extract_params_from_argument_tuple(
        &self,
        arg_tuple: Node<'ctx>,
    ) -> Vec<FunctionParam<'ctx>> {
        // ArgumentTuple -> Type -> (Tuple with TupleElements | single type)
        let type_node = match arg_tuple.child(0) {
            Some(n) if n.kind() == NodeKind::Type => n.child(0).unwrap_or(n),
            Some(n) => n,
            None => return Vec::new(),
        };

        if type_node.kind() == NodeKind::Tuple {
            type_node
                .children()
                .filter(|c| c.kind() == NodeKind::TupleElement)
                .map(FunctionParam::from_tuple_element)
                .collect()
        } else {
            // Single parameter (or empty)
            // Check if it's an empty tuple representation
            if type_node.num_children() == 0 && type_node.text().is_none() {
                Vec::new()
            } else {
                vec![FunctionParam {
                    label: None,
                    type_ref: TypeRef::new(type_node),
                    is_variadic: false,
                }]
            }
        }
    }

    fn extract_impl_params(&self) -> Vec<FunctionParam<'ctx>> {
        self.raw
            .children()
            .filter(|c| c.kind() == NodeKind::ImplParameter)
            .map(|c| {
                let type_node = c
                    .child_of_kind(NodeKind::Type)
                    .and_then(|t| t.child(0))
                    .unwrap_or(c);
                FunctionParam {
                    label: None,
                    type_ref: TypeRef::new(type_node),
                    is_variadic: false,
                }
            })
            .collect()
    }

    /// Get the return type of this function.
    pub fn return_type(&self) -> Option<TypeRef<'ctx>> {
        for child in self.raw.children() {
            if child.kind() == NodeKind::ReturnType {
                // ReturnType -> Type -> actual_type
                return child
                    .child(0)
                    .map(|t| TypeRef::new(t.child(0).unwrap_or(t)));
            }
            // ImplFunctionType has ImplResult
            if child.kind() == NodeKind::ImplResult {
                return child
                    .child_of_kind(NodeKind::Type)
                    .map(|t| TypeRef::new(t.child(0).unwrap_or(t)));
            }
        }
        None
    }

    /// Check if this function is async.
    pub fn is_async(&self) -> bool {
        self.raw
            .descendants()
            .any(|n| n.kind() == NodeKind::AsyncAnnotation)
    }

    /// Check if this function throws.
    pub fn is_throwing(&self) -> bool {
        self.raw.descendants().any(|n| {
            n.kind() == NodeKind::ThrowsAnnotation || n.kind() == NodeKind::TypedThrowsAnnotation
        })
    }

    /// Get the thrown error type if this function has typed throws.
    pub fn thrown_error_type(&self) -> Option<TypeRef<'ctx>> {
        self.raw
            .descendants()
            .find(|n| n.kind() == NodeKind::TypedThrowsAnnotation)
            .and_then(|n| n.child(0))
            .map(TypeRef::new)
    }

    /// Get the calling convention of this function.
    pub fn convention(&self) -> FunctionConvention {
        match self.raw.kind() {
            NodeKind::CFunctionPointer => FunctionConvention::C,
            NodeKind::ObjCBlock | NodeKind::EscapingObjCBlock => FunctionConvention::Block,
            NodeKind::ThinFunctionType => FunctionConvention::Thin,
            _ => {
                // Check for ImplFunctionConvention child
                for child in self.raw.children() {
                    if matches!(
                        child.kind(),
                        NodeKind::ImplFunctionConvention | NodeKind::ImplFunctionConventionName
                    ) && let Some(text) = child.text()
                    {
                        return match text {
                            "c" => FunctionConvention::C,
                            "block" => FunctionConvention::Block,
                            "thin" => FunctionConvention::Thin,
                            _ => FunctionConvention::Swift,
                        };
                    }
                }
                FunctionConvention::Swift
            }
        }
    }

    /// Check if this function type escapes.
    pub fn is_escaping(&self) -> bool {
        !matches!(
            self.raw.kind(),
            NodeKind::NoEscapeFunctionType | NodeKind::AutoClosureType
        )
    }

    /// Check if this is an autoclosure.
    pub fn is_autoclosure(&self) -> bool {
        matches!(
            self.raw.kind(),
            NodeKind::AutoClosureType | NodeKind::EscapingAutoClosureType
        )
    }

    /// Check if this function has a sending result.
    pub fn has_sending_result(&self) -> bool {
        self.raw
            .children()
            .any(|c| c.kind() == NodeKind::SendingResultFunctionType)
    }
}

impl std::fmt::Debug for FunctionType<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = f.debug_struct("FunctionType");
        s.field("parameters", &self.parameters());
        s.field("return_type", &self.return_type());
        s.field("is_async", &self.is_async());
        s.field("is_throwing", &self.is_throwing());
        if self.has_sending_result() {
            s.field("has_sending_result", &true);
        }
        s.field("convention", &self.convention());
        s.finish()
    }
}

/// A SIL implementation function type.
///
/// This represents a function type with detailed SIL-level information
/// including calling conventions, parameter passing conventions, and ownership.
#[derive(Clone, Copy)]
pub struct ImplFunctionType<'ctx> {
    raw: Node<'ctx>,
}

impl<'ctx> ImplFunctionType<'ctx> {
    /// Create an ImplFunctionType from a raw node.
    pub fn new(raw: Node<'ctx>) -> Self {
        Self { raw }
    }

    /// Get the underlying raw node.
    pub fn raw(&self) -> Node<'ctx> {
        self.raw
    }

    /// Get the callee convention (e.g., "@callee_guaranteed", "@callee_owned").
    pub fn callee_convention(&self) -> Option<&'ctx str> {
        self.raw
            .child_of_kind(NodeKind::ImplConvention)
            .and_then(|c| c.text())
    }

    /// Get the parameters with their conventions.
    pub fn parameters(&self) -> Vec<ImplParam<'ctx>> {
        self.raw
            .children()
            .filter(|c| c.kind() == NodeKind::ImplParameter)
            .map(ImplParam::new)
            .collect()
    }

    /// Get the results with their conventions.
    pub fn results(&self) -> Vec<ImplResult<'ctx>> {
        self.raw
            .children()
            .filter(|c| c.kind() == NodeKind::ImplResult)
            .map(ImplResult::new)
            .collect()
    }

    /// Get the error result if present.
    pub fn error_result(&self) -> Option<ImplResult<'ctx>> {
        self.raw
            .child_of_kind(NodeKind::ImplErrorResult)
            .map(ImplResult::new)
    }

    /// Get the generic signature if this is a substituted function type.
    pub fn generic_signature(&self) -> Option<GenericSignature<'ctx>> {
        self.raw
            .child_of_kind(NodeKind::DependentGenericSignature)
            .map(GenericSignature::new)
    }

    /// Get the substitution types (the "for <...>" part).
    pub fn substitutions(&self) -> Vec<TypeRef<'ctx>> {
        // Look for ImplPatternSubstitutions or ImplInvocationSubstitutions
        for child in self.raw.children() {
            if child.kind() == NodeKind::ImplPatternSubstitutions
                || child.kind() == NodeKind::ImplInvocationSubstitutions
            {
                return child
                    .children()
                    .filter(|c| c.kind() == NodeKind::Type)
                    .filter_map(|t| t.child(0))
                    .map(TypeRef::new)
                    .collect();
            }
        }
        Vec::new()
    }

    /// Check if this function is escaping.
    pub fn is_escaping(&self) -> bool {
        self.raw
            .children()
            .any(|c| c.kind() == NodeKind::ImplEscaping)
    }

    /// Check if this function has a sending result.
    pub fn has_sending_result(&self) -> bool {
        self.raw
            .children()
            .any(|c| c.kind() == NodeKind::ImplSendingResult)
    }
}

impl std::fmt::Debug for ImplFunctionType<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = f.debug_struct("ImplFunctionType");
        s.field("callee_convention", &self.callee_convention());
        s.field("is_escaping", &self.is_escaping());
        if self.has_sending_result() {
            s.field("has_sending_result", &true);
        }
        s.field("parameters", &self.parameters());
        s.field("results", &self.results());
        if let Some(err) = self.error_result() {
            s.field("error_result", &err);
        }
        if let Some(sig) = self.generic_signature() {
            s.field("generic_signature", &sig);
        }
        let subs = self.substitutions();
        if !subs.is_empty() {
            s.field("substitutions", &subs);
        }
        s.finish()
    }
}

/// A parameter in a SIL implementation function type.
#[derive(Clone, Copy)]
pub struct ImplParam<'ctx> {
    raw: Node<'ctx>,
}

impl<'ctx> ImplParam<'ctx> {
    fn new(raw: Node<'ctx>) -> Self {
        Self { raw }
    }

    /// Get the convention (e.g., "@guaranteed", "@owned", "@in", "@inout").
    pub fn convention(&self) -> Option<&'ctx str> {
        self.raw
            .child_of_kind(NodeKind::ImplConvention)
            .and_then(|c| c.text())
    }

    /// Get the type of this parameter.
    pub fn type_ref(&self) -> Option<TypeRef<'ctx>> {
        self.raw
            .child_of_kind(NodeKind::Type)
            .and_then(|t| t.child(0))
            .map(TypeRef::new)
    }

    /// Check if this parameter has the "sending" attribute.
    pub fn is_sending(&self) -> bool {
        self.raw
            .children()
            .any(|c| c.kind() == NodeKind::ImplParameterSending)
    }
}

impl std::fmt::Debug for ImplParam<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = f.debug_struct("ImplParam");
        s.field("convention", &self.convention());
        if self.is_sending() {
            s.field("is_sending", &true);
        }
        s.field("type_ref", &self.type_ref());
        s.finish()
    }
}

/// A result in a SIL implementation function type.
#[derive(Clone, Copy)]
pub struct ImplResult<'ctx> {
    raw: Node<'ctx>,
}

impl<'ctx> ImplResult<'ctx> {
    fn new(raw: Node<'ctx>) -> Self {
        Self { raw }
    }

    /// Get the convention (e.g., "@owned", "@out").
    pub fn convention(&self) -> Option<&'ctx str> {
        self.raw
            .child_of_kind(NodeKind::ImplConvention)
            .and_then(|c| c.text())
    }

    /// Get the type of this result.
    pub fn type_ref(&self) -> Option<TypeRef<'ctx>> {
        self.raw
            .child_of_kind(NodeKind::Type)
            .and_then(|t| t.child(0))
            .map(TypeRef::new)
    }
}

impl std::fmt::Debug for ImplResult<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ImplResult")
            .field("convention", &self.convention())
            .field("type_ref", &self.type_ref())
            .finish()
    }
}

/// A parameter in a function type.
#[derive(Debug)]
pub struct FunctionParam<'ctx> {
    /// The parameter label, if any.
    pub label: Option<&'ctx str>,
    /// The type of the parameter.
    pub type_ref: TypeRef<'ctx>,
    /// Whether this is a variadic parameter (T...).
    pub is_variadic: bool,
}

impl<'ctx> FunctionParam<'ctx> {
    fn from_tuple_element(node: Node<'ctx>) -> Self {
        let mut label = None;
        let mut type_node = None;
        let mut is_variadic = false;

        for child in node.children() {
            match child.kind() {
                NodeKind::TupleElementName => {
                    label = child.text();
                }
                NodeKind::Type => {
                    type_node = child.child(0).or(Some(child));
                }
                NodeKind::VariadicMarker => {
                    is_variadic = true;
                }
                _ => {
                    if type_node.is_none() {
                        type_node = Some(child);
                    }
                }
            }
        }

        FunctionParam {
            label,
            type_ref: TypeRef::new(type_node.unwrap_or(node)),
            is_variadic,
        }
    }
}

/// A tuple element.
#[derive(Clone, Copy)]
pub struct TupleElement<'ctx> {
    raw: Node<'ctx>,
}

impl<'ctx> TupleElement<'ctx> {
    /// Create a TupleElement from a raw node.
    pub fn new(raw: Node<'ctx>) -> Self {
        Self { raw }
    }

    /// Get the underlying raw node.
    pub fn raw(&self) -> Node<'ctx> {
        self.raw
    }

    /// Get the label of this tuple element, if any.
    pub fn label(&self) -> Option<&'ctx str> {
        self.raw
            .child_of_kind(NodeKind::TupleElementName)
            .and_then(|c| c.text())
    }

    /// Get the type of this tuple element.
    pub fn type_ref(&self) -> TypeRef<'ctx> {
        let type_node = self
            .raw
            .child_of_kind(NodeKind::Type)
            .and_then(|c| c.child(0))
            .unwrap_or(self.raw);
        TypeRef::new(type_node)
    }

    /// Check if this is a variadic element (T...).
    pub fn is_variadic(&self) -> bool {
        self.raw
            .children()
            .any(|c| c.kind() == NodeKind::VariadicMarker)
    }
}

impl std::fmt::Debug for TupleElement<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TupleElement")
            .field("label", &self.label())
            .field("type", &self.type_ref())
            .field("is_variadic", &self.is_variadic())
            .finish()
    }
}

/// The calling convention of a function type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FunctionConvention {
    /// Swift calling convention (default).
    Swift,
    /// C calling convention (`@convention(c)`).
    C,
    /// Objective-C block calling convention (`@convention(block)`).
    Block,
    /// Thin function (no context).
    Thin,
}

/// A generic signature describing generic parameters and their constraints.
#[derive(Clone, Copy)]
pub struct GenericSignature<'ctx> {
    raw: Node<'ctx>,
}

impl<'ctx> GenericSignature<'ctx> {
    /// Create a GenericSignature from a raw DependentGenericSignature node.
    pub fn new(raw: Node<'ctx>) -> Self {
        Self { raw }
    }

    /// Get the underlying raw node.
    pub fn raw(&self) -> Node<'ctx> {
        self.raw
    }

    /// Get the number of generic parameters at each depth level.
    ///
    /// Returns a vector where index 0 is depth 0 params, index 1 is depth 1 params, etc.
    pub fn param_counts(&self) -> Vec<u64> {
        self.raw
            .children()
            .filter(|c| c.kind() == NodeKind::DependentGenericParamCount)
            .filter_map(|c| c.index())
            .collect()
    }

    /// Get all generic requirements (constraints).
    pub fn requirements(&self) -> Vec<GenericRequirement<'ctx>> {
        self.raw
            .children()
            .filter_map(|c| match c.kind() {
                NodeKind::DependentGenericConformanceRequirement => {
                    Some(GenericRequirement::from_conformance_node(c))
                }
                NodeKind::DependentGenericSameTypeRequirement => {
                    Some(GenericRequirement::from_same_type_node(c))
                }
                NodeKind::DependentGenericLayoutRequirement => {
                    Some(GenericRequirement::from_layout_node(c))
                }
                _ => None,
            })
            .collect()
    }
}

impl std::fmt::Debug for GenericSignature<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GenericSignature")
            .field("param_counts", &self.param_counts())
            .field("requirements", &self.requirements())
            .finish()
    }
}

/// A generic requirement (constraint) on a generic parameter.
#[derive(Clone, Copy)]
pub struct GenericRequirement<'ctx> {
    kind: GenericRequirementKind<'ctx>,
}

impl<'ctx> GenericRequirement<'ctx> {
    fn from_conformance_node(node: Node<'ctx>) -> Self {
        // Structure: DependentGenericConformanceRequirement -> [Type (param), Type (protocol)]
        let mut children = node.children();
        let subject = children
            .next()
            .filter(|c| c.kind() == NodeKind::Type)
            .and_then(|c| c.child(0))
            .map(TypeRef::new);
        let constraint = children
            .next()
            .filter(|c| c.kind() == NodeKind::Type)
            .and_then(|c| c.child(0))
            .map(TypeRef::new);

        Self {
            kind: GenericRequirementKind::Conformance {
                subject,
                constraint,
            },
        }
    }

    fn from_same_type_node(node: Node<'ctx>) -> Self {
        // Structure: DependentGenericSameTypeRequirement -> [Type (first), Type (second)]
        let mut children = node.children();
        let first = children
            .next()
            .filter(|c| c.kind() == NodeKind::Type)
            .and_then(|c| c.child(0))
            .map(TypeRef::new);
        let second = children
            .next()
            .filter(|c| c.kind() == NodeKind::Type)
            .and_then(|c| c.child(0))
            .map(TypeRef::new);

        Self {
            kind: GenericRequirementKind::SameType { first, second },
        }
    }

    fn from_layout_node(node: Node<'ctx>) -> Self {
        // Structure: DependentGenericLayoutRequirement -> [Type (param), Identifier (layout)]
        let subject = node
            .child_of_kind(NodeKind::Type)
            .and_then(|c| c.child(0))
            .map(TypeRef::new);
        let layout = node
            .child_of_kind(NodeKind::Identifier)
            .and_then(|c| c.text());

        Self {
            kind: GenericRequirementKind::Layout { subject, layout },
        }
    }

    /// Get the kind of this requirement.
    pub fn kind(&self) -> &GenericRequirementKind<'ctx> {
        &self.kind
    }

    /// Get a display string for this requirement.
    pub fn display(&self) -> String {
        match &self.kind {
            GenericRequirementKind::Conformance {
                subject,
                constraint,
            } => {
                let subj = subject.map(|s| s.display()).unwrap_or_else(|| "?".into());
                let cons = constraint
                    .map(|c| c.display())
                    .unwrap_or_else(|| "?".into());
                format!("{subj}: {cons}")
            }
            GenericRequirementKind::SameType { first, second } => {
                let f = first.map(|s| s.display()).unwrap_or_else(|| "?".into());
                let s = second.map(|s| s.display()).unwrap_or_else(|| "?".into());
                format!("{f} == {s}")
            }
            GenericRequirementKind::Layout { subject, layout } => {
                let subj = subject.map(|s| s.display()).unwrap_or_else(|| "?".into());
                let lay = layout.unwrap_or("?");
                format!("{subj}: {lay}")
            }
        }
    }
}

impl std::fmt::Debug for GenericRequirement<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.kind {
            GenericRequirementKind::Conformance {
                subject,
                constraint,
            } => f
                .debug_struct("Conformance")
                .field("subject", subject)
                .field("constraint", constraint)
                .finish(),
            GenericRequirementKind::SameType { first, second } => f
                .debug_struct("SameType")
                .field("first", first)
                .field("second", second)
                .finish(),
            GenericRequirementKind::Layout { subject, layout } => f
                .debug_struct("Layout")
                .field("subject", subject)
                .field("layout", layout)
                .finish(),
        }
    }
}

impl std::fmt::Display for GenericRequirement<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display())
    }
}

/// The kind of generic requirement.
#[derive(Clone, Copy)]
pub enum GenericRequirementKind<'ctx> {
    /// A conformance requirement (`T: Protocol`).
    Conformance {
        subject: Option<TypeRef<'ctx>>,
        constraint: Option<TypeRef<'ctx>>,
    },
    /// A same-type requirement (`T == U` or `T == SomeType`).
    SameType {
        first: Option<TypeRef<'ctx>>,
        second: Option<TypeRef<'ctx>>,
    },
    /// A layout requirement (`T: _NativeClass`, etc.).
    Layout {
        subject: Option<TypeRef<'ctx>>,
        layout: Option<&'ctx str>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::raw::Context;

    /// Parse a type mangling and return the inner type.
    /// For manglings like `_TtGSqSS_`, navigates Global -> TypeMangling -> Type -> actual_type
    fn parse_type<'ctx>(ctx: &'ctx Context, mangled: &str) -> Option<TypeRef<'ctx>> {
        let root = Node::parse(ctx, mangled)?;
        // Navigate: Global -> first Type node -> child
        root.descendants()
            .find(|n| n.kind() == NodeKind::Type)
            .and_then(|n| n.child(0))
            .map(TypeRef::new)
    }

    #[test]
    fn test_simple_type() {
        let ctx = Context::new();
        // Swift.Int - this produces Global -> Structure directly
        let root = Node::parse(&ctx, "$sSi").unwrap();
        let type_ref = TypeRef::new(root.child(0).unwrap());
        assert!(matches!(type_ref.kind(), TypeKind::Named(_)));
    }

    #[test]
    fn test_function_type() {
        let ctx = Context::new();
        // (Swift.Int) -> Swift.String
        let type_ref = parse_type(&ctx, "_TtFSiSS").expect("Should parse");
        if let TypeKind::Function(func) = type_ref.kind() {
            assert!(!func.is_async());
            assert!(!func.is_throwing());
            assert_eq!(func.convention(), FunctionConvention::Swift);
        } else {
            panic!("Expected function type, got {:?}", type_ref.kind());
        }
    }

    #[test]
    fn test_optional_type() {
        let ctx = Context::new();
        // Swift.String? - produces BoundGenericEnum (Swift.Optional<String>)
        let type_ref = parse_type(&ctx, "_TtGSqSS_").expect("Should parse");
        // The classic mangling produces BoundGenericEnum, which we classify as Named
        // This is semantically correct - Optional is a named type
        if let TypeKind::Named(named) = type_ref.kind() {
            assert_eq!(named.name(), Some("Optional"));
            assert!(named.is_generic());
        } else {
            panic!(
                "Expected named type for Optional, got {:?}",
                type_ref.kind()
            );
        }
    }

    #[test]
    fn test_array_type() {
        let ctx = Context::new();
        // [Swift.String] - produces BoundGenericStructure (Swift.Array<String>)
        let type_ref = parse_type(&ctx, "_TtGSaSS_").expect("Should parse");
        // The classic mangling produces BoundGenericStructure
        if let TypeKind::Named(named) = type_ref.kind() {
            assert_eq!(named.name(), Some("Array"));
            assert!(named.is_generic());
        } else {
            panic!("Expected named type for Array, got {:?}", type_ref.kind());
        }
    }

    #[test]
    fn test_dictionary_type() {
        let ctx = Context::new();
        // [Swift.String : Swift.Int]
        let type_ref = parse_type(&ctx, "_TtGVs10DictionarySSSi_").expect("Should parse");
        // Produces BoundGenericStructure (Swift.Dictionary<String, Int>)
        if let TypeKind::Named(named) = type_ref.kind() {
            assert_eq!(named.name(), Some("Dictionary"));
            assert!(named.is_generic());
            let args = named.generic_args();
            assert_eq!(args.len(), 2);
        } else {
            panic!(
                "Expected named type for Dictionary, got {:?}",
                type_ref.kind()
            );
        }
    }

    #[test]
    fn test_c_function_pointer() {
        let ctx = Context::new();
        // @convention(c) (Swift.Int) -> Swift.UInt
        let type_ref = parse_type(&ctx, "_TtcSiSu").expect("Should parse");
        if let TypeKind::Function(func) = type_ref.kind() {
            assert_eq!(func.convention(), FunctionConvention::C);
        } else {
            panic!("Expected function type, got {:?}", type_ref.kind());
        }
    }

    #[test]
    fn test_block_type() {
        let ctx = Context::new();
        // @convention(block) (Swift.Int) -> Swift.UInt
        let type_ref = parse_type(&ctx, "_TtbSiSu").expect("Should parse");
        if let TypeKind::Function(func) = type_ref.kind() {
            assert_eq!(func.convention(), FunctionConvention::Block);
        } else {
            panic!("Expected function type, got {:?}", type_ref.kind());
        }
    }

    #[test]
    fn test_tuple_type() {
        let ctx = Context::new();
        // (Swift.Int, Swift.UInt)
        let type_ref = parse_type(&ctx, "_TtTSiSu_").expect("Should parse");
        if let TypeKind::Tuple(elements) = type_ref.kind() {
            assert_eq!(elements.len(), 2);
        } else {
            panic!("Expected tuple type, got {:?}", type_ref.kind());
        }
    }

    #[test]
    fn test_labeled_tuple() {
        let ctx = Context::new();
        // (foo: Swift.Int, bar: Swift.UInt)
        let type_ref = parse_type(&ctx, "_TtT3fooSi3barSu_").expect("Should parse");
        if let TypeKind::Tuple(elements) = type_ref.kind() {
            assert_eq!(elements.len(), 2);
            assert_eq!(elements[0].label(), Some("foo"));
            assert_eq!(elements[1].label(), Some("bar"));
        } else {
            panic!("Expected tuple type, got {:?}", type_ref.kind());
        }
    }

    #[test]
    fn test_metatype() {
        let ctx = Context::new();
        // Swift.Int.Type
        let type_ref = parse_type(&ctx, "_TtMSi").expect("Should parse");
        assert!(
            matches!(type_ref.kind(), TypeKind::Metatype(_)),
            "Expected metatype, got {:?}",
            type_ref.kind()
        );
    }

    #[test]
    fn test_inout_type() {
        let ctx = Context::new();
        // inout Swift.Int
        let type_ref = parse_type(&ctx, "_TtRSi").expect("Should parse");
        assert!(
            matches!(type_ref.kind(), TypeKind::InOut(_)),
            "Expected inout type, got {:?}",
            type_ref.kind()
        );
    }

    #[test]
    fn test_named_type_generic_args() {
        let ctx = Context::new();
        // Swift.Optional<Swift.String>
        let type_ref = parse_type(&ctx, "_TtGSqSS_").expect("Should parse");
        if let TypeKind::Named(named) = type_ref.kind() {
            let args = named.generic_args();
            assert_eq!(args.len(), 1);
            // The argument should be String
            assert!(args[0].display().contains("String"));
        } else {
            panic!("Expected named type");
        }
    }
}
