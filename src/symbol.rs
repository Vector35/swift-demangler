//! Top-level symbol representation.
//!
//! This module provides the main `Symbol` enum that categorizes demangled Swift symbols
//! into specific types for easier consumption.

use crate::accessor::Accessor;
use crate::async_symbol::AsyncSymbol;
use crate::autodiff::AutoDiff;
use crate::closure::Closure;
use crate::constructor::{Constructor, Destructor};
use crate::descriptor::Descriptor;
use crate::enum_case::EnumCase;
use crate::function::Function;
use crate::helpers::{HasModule, NodeExt};
use crate::macro_symbol::MacroSymbol;
use crate::metadata::Metadata;
use crate::outlined::Outlined;
use crate::raw::{Context, Node, NodeKind};
use crate::specialization::Specialization;
use crate::thunk::Thunk;
use crate::types::TypeRef;
use crate::witness_table::WitnessTable;

/// A parsed Swift symbol.
///
/// This enum categorizes symbols based on the first child of the `Global` root node.
/// It provides a high-level view of what a symbol represents.
#[derive(Debug)]
pub enum Symbol<'ctx> {
    /// A function symbol.
    Function(Function<'ctx>),
    /// A constructor (initializer).
    Constructor(Constructor<'ctx>),
    /// A destructor (deinitializer).
    Destructor(Destructor<'ctx>),
    /// An enum case constructor.
    EnumCase(EnumCase<'ctx>),
    /// A property accessor (getter, setter, etc.).
    Accessor(Accessor<'ctx>),
    /// A global variable.
    Variable(Variable<'ctx>),
    /// A closure.
    Closure(Closure<'ctx>),
    /// A thunk (dispatch, reabstraction, partial apply, etc.).
    Thunk(Thunk<'ctx>),
    /// A generic specialization (pre-compiled with specific type arguments).
    Specialization(SpecializedSymbol<'ctx>),
    /// A witness table (protocol or value).
    WitnessTable(WitnessTable<'ctx>),
    /// A metadata descriptor (protocol conformance, type, etc.).
    Descriptor(Descriptor<'ctx>),
    /// Type metadata (runtime type information).
    Metadata(Metadata<'ctx>),
    /// A type symbol (class, struct, enum, etc.).
    Type(TypeRef<'ctx>),
    /// A symbol with an attribute (@objc, @nonobjc, dynamic, distributed).
    Attributed(AttributedSymbol<'ctx>),
    /// A default argument initializer.
    DefaultArgument(DefaultArgument<'ctx>),
    /// An outlined operation (compiler-generated helpers).
    Outlined(OutlinedSymbol<'ctx>),
    /// An async/coroutine symbol.
    Async(AsyncSymbol<'ctx>),
    /// A macro or macro expansion.
    Macro(MacroSymbol<'ctx>),
    /// An automatic differentiation symbol.
    AutoDiff(AutoDiff<'ctx>),
    /// A bare identifier (name reference without full symbol info).
    Identifier(Node<'ctx>),
    /// A symbol with an unmangled suffix.
    Suffixed(SuffixedSymbol<'ctx>),
    /// Fallback for unhandled symbol kinds.
    Other(Node<'ctx>),
}

/// A symbol with an attribute modifier (@objc, @nonobjc, dynamic, distributed).
pub struct AttributedSymbol<'ctx> {
    /// The attribute kind.
    pub attribute: SymbolAttribute,
    /// The inner symbol.
    pub inner: Box<Symbol<'ctx>>,
    /// The raw attribute node.
    raw: Node<'ctx>,
}

impl<'ctx> AttributedSymbol<'ctx> {
    /// Get the raw attribute node.
    pub fn raw(&self) -> Node<'ctx> {
        self.raw
    }
}

impl std::fmt::Debug for AttributedSymbol<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AttributedSymbol")
            .field("attribute", &self.attribute)
            .field("inner", &self.inner)
            .finish()
    }
}

/// Symbol attributes that can be applied to declarations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolAttribute {
    /// `@objc` attribute - exposed to Objective-C.
    ObjC,
    /// `@nonobjc` attribute - not exposed to Objective-C.
    NonObjC,
    /// `dynamic` attribute - uses dynamic dispatch.
    Dynamic,
    /// `distributed` attribute - for distributed actors.
    Distributed,
}

impl SymbolAttribute {
    /// Get a human-readable name for this attribute.
    pub fn name(&self) -> &'static str {
        match self {
            SymbolAttribute::ObjC => "@objc",
            SymbolAttribute::NonObjC => "@nonobjc",
            SymbolAttribute::Dynamic => "dynamic",
            SymbolAttribute::Distributed => "distributed",
        }
    }
}

/// A default argument initializer.
pub struct DefaultArgument<'ctx> {
    raw: Node<'ctx>,
}

impl<'ctx> DefaultArgument<'ctx> {
    /// Create a new DefaultArgument.
    pub fn new(raw: Node<'ctx>) -> Self {
        Self { raw }
    }

    /// Get the raw node.
    pub fn raw(&self) -> Node<'ctx> {
        self.raw
    }

    /// Get the argument index (0-based).
    pub fn index(&self) -> Option<u64> {
        for child in self.raw.children() {
            if child.kind() == NodeKind::Number {
                return child.index();
            }
        }
        None
    }

    /// Get the function this default argument belongs to.
    pub fn function(&self) -> Option<Function<'ctx>> {
        for child in self.raw.children() {
            if child.kind() == NodeKind::Function {
                return Some(Function::new(child));
            }
        }
        None
    }

    /// Get the module containing this default argument.
    pub fn module(&self) -> Option<&'ctx str> {
        self.function().and_then(|f| f.module())
    }
}

impl std::fmt::Debug for DefaultArgument<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DefaultArgument")
            .field("index", &self.index())
            .field("function", &self.function())
            .field("module", &self.module())
            .finish()
    }
}

/// A specialized symbol containing both the specialization metadata and the inner symbol.
#[derive(Debug)]
pub struct SpecializedSymbol<'ctx> {
    /// The specialization metadata.
    pub specialization: Specialization<'ctx>,
    /// The inner symbol being specialized.
    pub inner: Box<Symbol<'ctx>>,
}

/// An outlined symbol containing both the outlined operation metadata and the context symbol.
#[derive(Debug)]
pub struct OutlinedSymbol<'ctx> {
    /// The outlined operation metadata.
    pub outlined: Outlined<'ctx>,
    /// The context symbol (usually a function) that this outlined operation is part of.
    pub context: Box<Symbol<'ctx>>,
}

/// A symbol with an unmangled suffix.
#[derive(Debug)]
pub struct SuffixedSymbol<'ctx> {
    /// The unmangled suffix text.
    pub suffix: &'ctx str,
    /// The inner symbol.
    pub inner: Box<Symbol<'ctx>>,
}

impl<'ctx> Symbol<'ctx> {
    /// Parse a mangled Swift symbol.
    ///
    /// Returns `None` if the symbol cannot be parsed.
    pub fn parse(ctx: &'ctx Context, mangled: &str) -> Option<Self> {
        let root = Node::parse(ctx, mangled)?;
        Self::from_node(root)
    }

    /// Create a Symbol from a parsed root node.
    ///
    /// The node should be a `Global` node (the root of a demangled symbol tree).
    pub fn from_node(root: Node<'ctx>) -> Option<Self> {
        if root.kind() != NodeKind::Global {
            return None;
        }

        let first_child = root.child(0)?;

        // Check if this is a specialization - they have the specialization metadata
        // as the first child and the inner symbol as subsequent children.
        // For nested specializations (spec of spec of func), all are siblings in Global.
        if Self::is_specialization_kind(first_child.kind()) {
            // Find the innermost non-specialization symbol and build the chain
            return Some(Self::build_specialization_chain(root, 0));
        }

        // Check if this is an outlined operation - they have the outlined node
        // as the first child and the context (usually a function) as the second child
        // Pattern: [Outlined, (Attribute)*, Function/Type]
        if Self::is_outlined_kind(first_child.kind()) {
            // Find the context node, skipping over any attributes and suffixes
            let mut context_idx = 1;
            let mut attr_nodes = Vec::new();
            while let Some(node) = root.child(context_idx) {
                if Self::get_attribute_kind(node.kind()).is_some() {
                    attr_nodes.push(node);
                    context_idx += 1;
                } else if node.kind() == NodeKind::Suffix {
                    // Skip suffix nodes - they're just metadata, not context
                    context_idx += 1;
                } else {
                    break;
                }
            }

            if let Some(context_node) = root.child(context_idx) {
                // Sibling pattern: [Outlined, (Attr)*, Function/Type]
                let mut context = Self::classify(context_node);
                // Wrap with attributes if present (in reverse order so outermost is first)
                for attr_node in attr_nodes.into_iter().rev() {
                    if let Some(attr) = Self::get_attribute_kind(attr_node.kind()) {
                        context = Symbol::Attributed(AttributedSymbol {
                            attribute: attr,
                            inner: Box::new(context),
                            raw: attr_node,
                        });
                    }
                }
                return Some(Symbol::Outlined(OutlinedSymbol {
                    outlined: Outlined::new(first_child),
                    context: Box::new(context),
                }));
            }
            // Child pattern: Outlined has Type as child (no sibling)
            // Try to extract a type from the outlined node's children
            if let Some(type_ref) = first_child.extract_type_ref() {
                return Some(Symbol::Outlined(OutlinedSymbol {
                    outlined: Outlined::new(first_child),
                    context: Box::new(Symbol::Type(type_ref)),
                }));
            }
            // No context found at all
            return Some(Symbol::Outlined(OutlinedSymbol {
                outlined: Outlined::new(first_child),
                context: Box::new(Symbol::Other(first_child)),
            }));
        }

        // Check for sibling modifier patterns where the first child is an attribute
        // and the second child is the actual symbol (e.g., @objc, @nonobjc, dynamic)
        if let Some(attribute) = Self::get_attribute_kind(first_child.kind())
            && let Some(inner_node) = root.child(1)
        {
            let inner = Self::classify(inner_node);
            return Some(Symbol::Attributed(AttributedSymbol {
                attribute,
                inner: Box::new(inner),
                raw: first_child,
            }));
        }

        // Check for marker kinds (async/coro, metadata, thunk markers) that have the actual symbol as a sibling
        // These can be chained: [CoroFunctionPointer, DefaultOverride, Accessor]
        if Self::is_async_marker_kind(first_child.kind())
            || Self::is_metadata_marker_kind(first_child.kind())
            || Self::is_thunk_marker_kind(first_child.kind())
        {
            return Some(Self::build_marker_chain(root, 0));
        }

        // Check for Suffix sibling and wrap the symbol if present
        let suffix = root.child_of_kind(NodeKind::Suffix).and_then(|c| c.text());

        let inner = Self::classify(first_child);

        if let Some(suffix) = suffix {
            Some(Symbol::Suffixed(SuffixedSymbol {
                suffix,
                inner: Box::new(inner),
            }))
        } else {
            Some(inner)
        }
    }

    fn is_specialization_kind(kind: NodeKind) -> bool {
        matches!(
            kind,
            NodeKind::GenericSpecialization
                | NodeKind::GenericSpecializationNotReAbstracted
                | NodeKind::GenericSpecializationInResilienceDomain
                | NodeKind::GenericSpecializationPrespecialized
                | NodeKind::GenericPartialSpecialization
                | NodeKind::GenericPartialSpecializationNotReAbstracted
                | NodeKind::FunctionSignatureSpecialization
        )
    }

    /// Build a chain of nested specializations.
    ///
    /// For nested specializations like `spec of spec of func`, all nodes are
    /// siblings in Global: [Spec1, Spec2, Func]. This function recursively
    /// builds the nested SpecializedSymbol structure.
    fn build_specialization_chain(root: Node<'ctx>, index: usize) -> Symbol<'ctx> {
        let Some(current) = root.child(index) else {
            // No more children - shouldn't happen for well-formed symbols
            return Symbol::Other(root);
        };

        if Self::is_specialization_kind(current.kind()) {
            // This is a specialization - its inner is the next sibling
            let inner = if let Some(next) = root.child(index + 1) {
                if Self::is_specialization_kind(next.kind()) {
                    // Next sibling is also a specialization - recurse
                    Self::build_specialization_chain(root, index + 1)
                } else {
                    // Next sibling is the actual function/symbol
                    Self::classify(next)
                }
            } else {
                // No next sibling - use current as fallback
                Symbol::Other(current)
            };

            Symbol::Specialization(SpecializedSymbol {
                specialization: Specialization::new(current),
                inner: Box::new(inner),
            })
        } else {
            // Not a specialization - just classify it
            Self::classify(current)
        }
    }

    /// Build a chain of nested marker symbols (async/coro pointers, default overrides, thunk markers).
    ///
    /// For chains like `[CoroFunctionPointer, DefaultOverride, Accessor]`, this builds
    /// nested wrapper symbols.
    fn build_marker_chain(root: Node<'ctx>, index: usize) -> Symbol<'ctx> {
        let Some(current) = root.child(index) else {
            return Symbol::Other(root);
        };

        // Get the inner symbol (either another marker or the actual symbol)
        let inner = if let Some(next) = root.child(index + 1) {
            if Self::is_async_marker_kind(next.kind())
                || Self::is_metadata_marker_kind(next.kind())
                || Self::is_thunk_marker_kind(next.kind())
            {
                // Next sibling is also a marker - recurse
                Self::build_marker_chain(root, index + 1)
            } else {
                // Next sibling is the actual symbol
                Self::classify(next)
            }
        } else {
            // No next sibling - use current as fallback
            return Symbol::Other(current);
        };

        // Wrap the inner symbol with the appropriate marker
        if Self::is_async_marker_kind(current.kind()) {
            Symbol::Async(AsyncSymbol::with_inner(current, inner))
        } else if Self::is_metadata_marker_kind(current.kind()) {
            Symbol::Metadata(Metadata::with_inner(current, inner))
        } else if Self::is_thunk_marker_kind(current.kind()) {
            Symbol::Thunk(Thunk::new_marker(current, inner))
        } else {
            // Shouldn't happen, but fall back to the inner symbol
            inner
        }
    }

    fn is_outlined_kind(kind: NodeKind) -> bool {
        matches!(
            kind,
            NodeKind::OutlinedBridgedMethod
                | NodeKind::OutlinedVariable
                | NodeKind::OutlinedCopy
                | NodeKind::OutlinedConsume
                | NodeKind::OutlinedRetain
                | NodeKind::OutlinedRelease
                | NodeKind::OutlinedInitializeWithTake
                | NodeKind::OutlinedInitializeWithCopy
                | NodeKind::OutlinedAssignWithTake
                | NodeKind::OutlinedAssignWithCopy
                | NodeKind::OutlinedDestroy
                | NodeKind::OutlinedInitializeWithCopyNoValueWitness
                | NodeKind::OutlinedInitializeWithTakeNoValueWitness
                | NodeKind::OutlinedAssignWithCopyNoValueWitness
                | NodeKind::OutlinedAssignWithTakeNoValueWitness
                | NodeKind::OutlinedDestroyNoValueWitness
                | NodeKind::OutlinedReadOnlyObject
        )
    }

    fn get_attribute_kind(kind: NodeKind) -> Option<SymbolAttribute> {
        match kind {
            NodeKind::ObjCAttribute => Some(SymbolAttribute::ObjC),
            NodeKind::NonObjCAttribute => Some(SymbolAttribute::NonObjC),
            NodeKind::DynamicAttribute => Some(SymbolAttribute::Dynamic),
            NodeKind::DistributedAccessor => Some(SymbolAttribute::Distributed),
            _ => None,
        }
    }

    fn is_async_marker_kind(kind: NodeKind) -> bool {
        matches!(
            kind,
            NodeKind::CoroFunctionPointer
                | NodeKind::AsyncFunctionPointer
                | NodeKind::AsyncAwaitResumePartialFunction
                | NodeKind::AsyncSuspendResumePartialFunction
        )
    }

    fn is_metadata_marker_kind(kind: NodeKind) -> bool {
        matches!(kind, NodeKind::DefaultOverride | NodeKind::HasSymbolQuery)
    }

    fn is_thunk_marker_kind(kind: NodeKind) -> bool {
        matches!(
            kind,
            NodeKind::InlinedGenericFunction
                | NodeKind::MergedFunction
                | NodeKind::DistributedThunk
        )
    }

    fn classify(node: Node<'ctx>) -> Self {
        Self::classify_with_static(node, false)
    }

    /// Classify a node into a Symbol.
    ///
    /// This is used internally and by Thunk::inner() to classify wrapped symbols.
    pub fn classify_node(node: Node<'ctx>) -> Self {
        Self::classify_with_static(node, false)
    }

    fn classify_with_static(node: Node<'ctx>, is_static: bool) -> Self {
        match node.kind() {
            // Static wrapper - recurse with static flag
            NodeKind::Static => {
                if let Some(inner) = node.child(0) {
                    return Self::classify_with_static(inner, true);
                }
                Symbol::Other(node)
            }

            // Functions
            NodeKind::Function => {
                if is_static {
                    Symbol::Function(Function::new_static(node))
                } else {
                    Symbol::Function(Function::new(node))
                }
            }

            // Constructors
            NodeKind::Constructor | NodeKind::Allocator => {
                Symbol::Constructor(Constructor::new(node))
            }

            // Destructors
            NodeKind::Destructor | NodeKind::Deallocator | NodeKind::IsolatedDeallocator => {
                Symbol::Destructor(Destructor::new(node))
            }

            // Enum cases
            NodeKind::EnumCase => Symbol::EnumCase(EnumCase::new(node)),

            // Accessors
            NodeKind::Getter
            | NodeKind::Setter
            | NodeKind::ModifyAccessor
            | NodeKind::Modify2Accessor
            | NodeKind::ReadAccessor
            | NodeKind::Read2Accessor
            | NodeKind::WillSet
            | NodeKind::DidSet
            | NodeKind::GlobalGetter
            | NodeKind::MaterializeForSet
            | NodeKind::InitAccessor
            | NodeKind::UnsafeAddressor
            | NodeKind::UnsafeMutableAddressor
            | NodeKind::OwningAddressor
            | NodeKind::OwningMutableAddressor
            | NodeKind::NativeOwningAddressor
            | NodeKind::NativeOwningMutableAddressor
            | NodeKind::NativePinningAddressor
            | NodeKind::NativePinningMutableAddressor => {
                if is_static {
                    Symbol::Accessor(Accessor::new_static(node))
                } else {
                    Symbol::Accessor(Accessor::new(node))
                }
            }

            // Variables
            NodeKind::Variable => Symbol::Variable(Variable::new(node)),

            // Closures
            NodeKind::ExplicitClosure | NodeKind::ImplicitClosure => {
                Symbol::Closure(Closure::new(node))
            }

            // Thunks and optimization-related wrappers
            NodeKind::DispatchThunk
            | NodeKind::VTableThunk
            | NodeKind::DistributedThunk
            | NodeKind::ReabstractionThunk
            | NodeKind::ReabstractionThunkHelper
            | NodeKind::ReabstractionThunkHelperWithSelf
            | NodeKind::ReabstractionThunkHelperWithGlobalActor
            | NodeKind::AutoDiffSelfReorderingReabstractionThunk
            | NodeKind::PartialApplyForwarder
            | NodeKind::PartialApplyObjCForwarder
            | NodeKind::CurryThunk
            | NodeKind::ProtocolWitness
            | NodeKind::ProtocolSelfConformanceWitness
            | NodeKind::KeyPathGetterThunkHelper
            | NodeKind::KeyPathSetterThunkHelper
            | NodeKind::KeyPathUnappliedMethodThunkHelper
            | NodeKind::KeyPathAppliedMethodThunkHelper
            | NodeKind::KeyPathEqualsThunkHelper
            | NodeKind::KeyPathHashThunkHelper
            | NodeKind::AutoDiffSubsetParametersThunk
            | NodeKind::AutoDiffDerivativeVTableThunk
            | NodeKind::BackDeploymentThunk
            | NodeKind::BackDeploymentFallback
            | NodeKind::MergedFunction
            | NodeKind::InlinedGenericFunction => Symbol::Thunk(Thunk::new(node)),

            // Witness tables and value witnesses
            NodeKind::ProtocolWitnessTable
            | NodeKind::ProtocolWitnessTableAccessor
            | NodeKind::ProtocolWitnessTablePattern
            | NodeKind::GenericProtocolWitnessTable
            | NodeKind::GenericProtocolWitnessTableInstantiationFunction
            | NodeKind::ResilientProtocolWitnessTable
            | NodeKind::LazyProtocolWitnessTableAccessor
            | NodeKind::LazyProtocolWitnessTableCacheVariable
            | NodeKind::ProtocolSelfConformanceWitnessTable
            | NodeKind::ValueWitnessTable
            | NodeKind::ValueWitness
            | NodeKind::AssociatedTypeWitnessTableAccessor
            | NodeKind::BaseWitnessTableAccessor
            | NodeKind::ConcreteProtocolConformance => {
                Symbol::WitnessTable(WitnessTable::new(node))
            }

            // Metadata descriptors
            NodeKind::ProtocolConformanceDescriptor
            | NodeKind::ProtocolConformanceDescriptorRecord
            | NodeKind::OpaqueTypeDescriptor
            | NodeKind::OpaqueTypeDescriptorRecord
            | NodeKind::OpaqueTypeDescriptorAccessor
            | NodeKind::OpaqueTypeDescriptorAccessorImpl
            | NodeKind::OpaqueTypeDescriptorAccessorKey
            | NodeKind::OpaqueTypeDescriptorAccessorVar
            | NodeKind::NominalTypeDescriptor
            | NodeKind::NominalTypeDescriptorRecord
            | NodeKind::PropertyDescriptor
            | NodeKind::ProtocolDescriptor
            | NodeKind::ProtocolDescriptorRecord
            | NodeKind::ProtocolRequirementsBaseDescriptor
            | NodeKind::MethodDescriptor
            | NodeKind::AssociatedTypeDescriptor
            | NodeKind::AssociatedConformanceDescriptor
            | NodeKind::DefaultAssociatedConformanceAccessor
            | NodeKind::BaseConformanceDescriptor
            | NodeKind::ExtensionDescriptor
            | NodeKind::AnonymousDescriptor
            | NodeKind::ModuleDescriptor
            | NodeKind::ReflectionMetadataAssocTypeDescriptor
            | NodeKind::AccessibleFunctionRecord => Symbol::Descriptor(Descriptor::new(node)),

            // Type metadata
            NodeKind::TypeMetadata
            | NodeKind::FullTypeMetadata
            | NodeKind::TypeMetadataAccessFunction
            | NodeKind::TypeMetadataCompletionFunction
            | NodeKind::TypeMetadataInstantiationFunction
            | NodeKind::TypeMetadataInstantiationCache
            | NodeKind::TypeMetadataLazyCache
            | NodeKind::TypeMetadataSingletonInitializationCache
            | NodeKind::TypeMetadataDemanglingCache
            | NodeKind::GenericTypeMetadataPattern
            | NodeKind::MetadataInstantiationCache
            | NodeKind::NoncanonicalSpecializedGenericTypeMetadata
            | NodeKind::NoncanonicalSpecializedGenericTypeMetadataCache
            | NodeKind::CanonicalSpecializedGenericTypeMetadataAccessFunction
            | NodeKind::AssociatedTypeMetadataAccessor
            | NodeKind::DefaultAssociatedTypeMetadataAccessor
            | NodeKind::ClassMetadataBaseOffset
            | NodeKind::ObjCMetadataUpdateFunction
            | NodeKind::FieldOffset
            | NodeKind::Metaclass
            | NodeKind::IVarInitializer
            | NodeKind::IVarDestroyer
            | NodeKind::HasSymbolQuery
            | NodeKind::DefaultOverride
            | NodeKind::PropertyWrapperBackingInitializer
            | NodeKind::MethodLookupFunction => Symbol::Metadata(Metadata::new(node)),

            // Default argument initializers
            NodeKind::DefaultArgumentInitializer => {
                Symbol::DefaultArgument(DefaultArgument::new(node))
            }

            // Type symbols
            NodeKind::TypeMangling => {
                if let Some(type_node) = node.child(0) {
                    Symbol::Type(TypeRef::new(type_node))
                } else {
                    Symbol::Other(node)
                }
            }

            // Named types as symbols
            NodeKind::Class
            | NodeKind::Structure
            | NodeKind::Enum
            | NodeKind::Protocol
            | NodeKind::TypeAlias
            | NodeKind::OtherNominalType
            | NodeKind::BuiltinTypeName
            | NodeKind::BoundGenericStructure
            | NodeKind::BoundGenericClass
            | NodeKind::BoundGenericEnum
            | NodeKind::Tuple
            | NodeKind::BuiltinFixedArray => Symbol::Type(TypeRef::new(node)),

            // Subscript is an accessor
            NodeKind::Subscript => {
                if is_static {
                    Symbol::Accessor(Accessor::new_static(node))
                } else {
                    Symbol::Accessor(Accessor::new(node))
                }
            }

            // Note: Outlined operations are handled in from_node() as sibling patterns,
            // not here in classify_with_static()

            // Async and coroutine symbols
            NodeKind::AsyncAwaitResumePartialFunction
            | NodeKind::AsyncSuspendResumePartialFunction
            | NodeKind::AsyncFunctionPointer
            | NodeKind::CoroFunctionPointer
            | NodeKind::CoroutineContinuationPrototype => Symbol::Async(AsyncSymbol::new(node)),

            // Macro symbols
            NodeKind::Macro
            | NodeKind::FreestandingMacroExpansion
            | NodeKind::MacroExpansionUniqueName
            | NodeKind::AccessorAttachedMacroExpansion
            | NodeKind::BodyAttachedMacroExpansion
            | NodeKind::ConformanceAttachedMacroExpansion
            | NodeKind::ExtensionAttachedMacroExpansion
            | NodeKind::MemberAttachedMacroExpansion
            | NodeKind::MemberAttributeAttachedMacroExpansion
            | NodeKind::PeerAttachedMacroExpansion
            | NodeKind::PreambleAttachedMacroExpansion => Symbol::Macro(MacroSymbol::new(node)),

            // Automatic differentiation symbols
            NodeKind::AutoDiffFunction | NodeKind::DifferentiabilityWitness => {
                Symbol::AutoDiff(AutoDiff::new(node))
            }

            // Bare identifier (e.g., function name reference in specializations)
            NodeKind::Identifier => Symbol::Identifier(node),

            // Fallback
            _ => Symbol::Other(node),
        }
    }

    /// Get the raw node for this symbol.
    pub fn raw(&self) -> Node<'ctx> {
        match self {
            Symbol::Function(f) => f.raw(),
            Symbol::Constructor(c) => c.raw(),
            Symbol::Destructor(d) => d.raw(),
            Symbol::EnumCase(e) => e.raw(),
            Symbol::Accessor(a) => a.raw(),
            Symbol::Variable(v) => v.raw(),
            Symbol::Closure(c) => c.raw(),
            Symbol::Thunk(t) => t.raw(),
            Symbol::Specialization(s) => s.specialization.raw(),
            Symbol::WitnessTable(w) => w.raw(),
            Symbol::Descriptor(d) => d.raw(),
            Symbol::Metadata(m) => m.raw(),
            Symbol::Type(t) => t.raw(),
            Symbol::Attributed(a) => a.raw,
            Symbol::DefaultArgument(d) => d.raw(),
            Symbol::Outlined(o) => o.outlined.raw(),
            Symbol::Async(a) => a.raw(),
            Symbol::Macro(m) => m.raw(),
            Symbol::AutoDiff(a) => a.raw(),
            Symbol::Identifier(n) => *n,
            Symbol::Suffixed(s) => s.inner.raw(),
            Symbol::Other(n) => *n,
        }
    }

    /// Get the display string for this symbol.
    pub fn display(&self) -> String {
        self.raw().to_string()
    }

    /// Check if this is a function symbol.
    pub fn is_function(&self) -> bool {
        matches!(self, Symbol::Function(_))
    }

    /// Check if this is a constructor symbol.
    pub fn is_constructor(&self) -> bool {
        matches!(self, Symbol::Constructor(_))
    }

    /// Check if this is a destructor symbol.
    pub fn is_destructor(&self) -> bool {
        matches!(self, Symbol::Destructor(_))
    }

    /// Check if this is an enum case symbol.
    pub fn is_enum_case(&self) -> bool {
        matches!(self, Symbol::EnumCase(_))
    }

    /// Check if this is an accessor symbol.
    pub fn is_accessor(&self) -> bool {
        matches!(self, Symbol::Accessor(_))
    }

    /// Check if this is a variable symbol.
    pub fn is_variable(&self) -> bool {
        matches!(self, Symbol::Variable(_))
    }

    /// Check if this is a closure symbol.
    pub fn is_closure(&self) -> bool {
        matches!(self, Symbol::Closure(_))
    }

    /// Check if this is a type symbol.
    pub fn is_type(&self) -> bool {
        matches!(self, Symbol::Type(_))
    }

    /// Check if this is a thunk symbol.
    pub fn is_thunk(&self) -> bool {
        matches!(self, Symbol::Thunk(_))
    }

    /// Check if this is a specialization symbol.
    pub fn is_specialization(&self) -> bool {
        matches!(self, Symbol::Specialization(_))
    }

    /// Check if this is a witness table symbol.
    pub fn is_witness_table(&self) -> bool {
        matches!(self, Symbol::WitnessTable(_))
    }

    /// Check if this is a descriptor symbol.
    pub fn is_descriptor(&self) -> bool {
        matches!(self, Symbol::Descriptor(_))
    }

    /// Check if this is a metadata symbol.
    pub fn is_metadata(&self) -> bool {
        matches!(self, Symbol::Metadata(_))
    }

    /// Check if this is an attributed symbol.
    pub fn is_attributed(&self) -> bool {
        matches!(self, Symbol::Attributed(_))
    }

    /// Check if this is a default argument symbol.
    pub fn is_default_argument(&self) -> bool {
        matches!(self, Symbol::DefaultArgument(_))
    }

    /// Try to get this as a function.
    pub fn as_function(&self) -> Option<&Function<'ctx>> {
        match self {
            Symbol::Function(f) => Some(f),
            _ => None,
        }
    }

    /// Try to get this as a constructor.
    pub fn as_constructor(&self) -> Option<&Constructor<'ctx>> {
        match self {
            Symbol::Constructor(c) => Some(c),
            _ => None,
        }
    }

    /// Try to get this as a destructor.
    pub fn as_destructor(&self) -> Option<&Destructor<'ctx>> {
        match self {
            Symbol::Destructor(d) => Some(d),
            _ => None,
        }
    }

    /// Try to get this as an enum case.
    pub fn as_enum_case(&self) -> Option<&EnumCase<'ctx>> {
        match self {
            Symbol::EnumCase(e) => Some(e),
            _ => None,
        }
    }

    /// Try to get this as an accessor.
    pub fn as_accessor(&self) -> Option<&Accessor<'ctx>> {
        match self {
            Symbol::Accessor(a) => Some(a),
            _ => None,
        }
    }

    /// Try to get this as a variable.
    pub fn as_variable(&self) -> Option<&Variable<'ctx>> {
        match self {
            Symbol::Variable(v) => Some(v),
            _ => None,
        }
    }

    /// Try to get this as a closure.
    pub fn as_closure(&self) -> Option<&Closure<'ctx>> {
        match self {
            Symbol::Closure(c) => Some(c),
            _ => None,
        }
    }

    /// Try to get this as a type.
    pub fn as_type(&self) -> Option<&TypeRef<'ctx>> {
        match self {
            Symbol::Type(t) => Some(t),
            _ => None,
        }
    }

    /// Try to get this as a thunk.
    pub fn as_thunk(&self) -> Option<&Thunk<'ctx>> {
        match self {
            Symbol::Thunk(t) => Some(t),
            _ => None,
        }
    }

    /// Try to get this as a specialization.
    pub fn as_specialization(&self) -> Option<&SpecializedSymbol<'ctx>> {
        match self {
            Symbol::Specialization(s) => Some(s),
            _ => None,
        }
    }

    /// Try to get this as a witness table.
    pub fn as_witness_table(&self) -> Option<&WitnessTable<'ctx>> {
        match self {
            Symbol::WitnessTable(w) => Some(w),
            _ => None,
        }
    }

    /// Try to get this as a descriptor.
    pub fn as_descriptor(&self) -> Option<&Descriptor<'ctx>> {
        match self {
            Symbol::Descriptor(d) => Some(d),
            _ => None,
        }
    }

    /// Try to get this as metadata.
    pub fn as_metadata(&self) -> Option<&Metadata<'ctx>> {
        match self {
            Symbol::Metadata(m) => Some(m),
            _ => None,
        }
    }

    /// Try to get this as an attributed symbol.
    pub fn as_attributed(&self) -> Option<&AttributedSymbol<'ctx>> {
        match self {
            Symbol::Attributed(a) => Some(a),
            _ => None,
        }
    }

    /// Try to get this as a default argument.
    pub fn as_default_argument(&self) -> Option<&DefaultArgument<'ctx>> {
        match self {
            Symbol::DefaultArgument(d) => Some(d),
            _ => None,
        }
    }

    /// Check if this is an outlined operation symbol.
    pub fn is_outlined(&self) -> bool {
        matches!(self, Symbol::Outlined(_))
    }

    /// Check if this is an async symbol.
    pub fn is_async(&self) -> bool {
        matches!(self, Symbol::Async(_))
    }

    /// Check if this is a macro symbol.
    pub fn is_macro(&self) -> bool {
        matches!(self, Symbol::Macro(_))
    }

    /// Check if this is an auto-diff symbol.
    pub fn is_autodiff(&self) -> bool {
        matches!(self, Symbol::AutoDiff(_))
    }

    /// Try to get this as an outlined operation.
    pub fn as_outlined(&self) -> Option<&OutlinedSymbol<'ctx>> {
        match self {
            Symbol::Outlined(o) => Some(o),
            _ => None,
        }
    }

    /// Try to get this as an async symbol.
    pub fn as_async(&self) -> Option<&AsyncSymbol<'ctx>> {
        match self {
            Symbol::Async(a) => Some(a),
            _ => None,
        }
    }

    /// Try to get this as a macro.
    pub fn as_macro(&self) -> Option<&MacroSymbol<'ctx>> {
        match self {
            Symbol::Macro(m) => Some(m),
            _ => None,
        }
    }

    /// Try to get this as an auto-diff symbol.
    pub fn as_autodiff(&self) -> Option<&AutoDiff<'ctx>> {
        match self {
            Symbol::AutoDiff(a) => Some(a),
            _ => None,
        }
    }
}

impl std::fmt::Display for Symbol<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display())
    }
}

/// A global variable symbol.
#[derive(Clone, Copy)]
pub struct Variable<'ctx> {
    raw: Node<'ctx>,
}

impl<'ctx> Variable<'ctx> {
    /// Create a Variable from a raw node.
    pub fn new(raw: Node<'ctx>) -> Self {
        Self { raw }
    }

    /// Get the underlying raw node.
    pub fn raw(&self) -> Node<'ctx> {
        self.raw
    }

    /// Get the name of this variable.
    pub fn name(&self) -> Option<&'ctx str> {
        for child in self.raw.children() {
            if child.kind() == NodeKind::Identifier {
                return child.text();
            }
        }
        None
    }

    /// Get the module containing this variable.
    pub fn module(&self) -> Option<&'ctx str> {
        for child in self.raw.children() {
            if child.kind() == NodeKind::Module {
                return child.text();
            }
        }
        // Check nested context
        for child in self.raw.children() {
            match child.kind() {
                NodeKind::Class | NodeKind::Structure | NodeKind::Enum | NodeKind::Extension => {
                    for inner in child.descendants() {
                        if inner.kind() == NodeKind::Module {
                            return inner.text();
                        }
                    }
                }
                _ => {}
            }
        }
        None
    }

    /// Get the type of this variable.
    pub fn variable_type(&self) -> Option<TypeRef<'ctx>> {
        self.raw.extract_type_ref()
    }

    /// Get the context where this variable is defined.
    pub fn context(&self) -> crate::context::SymbolContext<'ctx> {
        crate::context::extract_context(self.raw)
    }
}

impl std::fmt::Debug for Variable<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Variable")
            .field("name", &self.name())
            .field("module", &self.module())
            .field("type", &self.variable_type())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_function() {
        let ctx = Context::new();
        let symbol = Symbol::parse(&ctx, "$s4main5helloSSyYaKF").unwrap();
        assert!(symbol.is_function());
    }

    #[test]
    fn test_parse_variable() {
        let ctx = Context::new();
        // foo.bar : Swift.Int
        let symbol = Symbol::parse(&ctx, "_Tv3foo3barSi").unwrap();
        assert!(symbol.is_variable());
        if let Symbol::Variable(var) = symbol {
            assert_eq!(var.name(), Some("bar"));
            assert_eq!(var.module(), Some("foo"));
        }
    }

    #[test]
    fn test_parse_getter() {
        let ctx = Context::new();
        // foo.bar.getter : Swift.Int
        let symbol = Symbol::parse(&ctx, "_TF3foog3barSi").unwrap();
        assert!(symbol.is_accessor());
    }

    #[test]
    fn test_parse_setter() {
        let ctx = Context::new();
        // foo.bar.setter : Swift.Int
        let symbol = Symbol::parse(&ctx, "_TF3foos3barSi").unwrap();
        assert!(symbol.is_accessor());
    }

    #[test]
    fn test_symbol_display() {
        let ctx = Context::new();
        let symbol = Symbol::parse(&ctx, "$s4main5helloSSyYaKF").unwrap();
        let display = symbol.display();
        assert!(display.contains("hello"));
    }

    #[test]
    fn test_symbol_as_methods() {
        let ctx = Context::new();
        let symbol = Symbol::parse(&ctx, "$s4main5helloSSyYaKF").unwrap();
        assert!(symbol.as_function().is_some());
        assert!(symbol.as_constructor().is_none());
        assert!(symbol.as_accessor().is_none());
    }
}
