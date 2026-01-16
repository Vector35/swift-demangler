//! Swift symbol demangling library with high-level semantic types.
//!
//! This crate provides both low-level (`raw`) and high-level APIs for working
//! with Swift mangled symbols.
//!
//! # High-Level API
//!
//! The high-level API provides semantic types that make it easy to work with
//! demangled symbols. Create a [`Context`] and use [`Symbol::parse`] to
//! get started:
//!
//! ```
//! use swift_demangler::{Context, HasFunctionSignature, HasModule, Symbol};
//!
//! let ctx = Context::new();
//! if let Some(symbol) = Symbol::parse(&ctx, "$s4main5helloSSyYaKF") {
//!     if let Some(func) = symbol.as_function() {
//!         println!("Function: {}", func.name().unwrap_or("?"));
//!         println!("Module: {}", func.module().unwrap_or("?"));
//!         println!("Async: {}", func.is_async());
//!         println!("Throws: {}", func.is_throwing());
//!     }
//! }
//! ```
//!
//! # Low-Level API
//!
//! The [`raw`] module provides direct access to the node tree:
//!
//! ```
//! use swift_demangler::Context;
//! use swift_demangler::raw::{Node, NodeKind};
//!
//! let ctx = Context::new();
//! if let Some(root) = Node::parse(&ctx, "$s4main5helloSSyYaKF") {
//!     for node in root.descendants() {
//!         println!("{:?}: {:?}", node.kind(), node.text());
//!     }
//! }
//! ```

// Low-level raw API
pub mod raw;

// Shared helper utilities
mod helpers;
pub use helpers::{
    HasExtensionContext, HasFunctionSignature, HasGenericSignature, HasModule,
};

// High-level semantic API modules
mod accessor;
mod async_symbol;
mod autodiff;
mod closure;
mod constructor;
mod context;
mod descriptor;
mod enum_case;
mod function;
mod macro_symbol;
mod metadata;
mod outlined;
mod specialization;
mod symbol;
mod thunk;
mod types;
mod witness_table;

// Re-export core types from raw module
pub use raw::{demangle, Context};

// Re-export high-level types at crate root
pub use accessor::{Accessor, AccessorKind};
pub use async_symbol::{AsyncSymbol, AsyncSymbolKind};
pub use autodiff::{AutoDiff, AutoDiffKind};
pub use closure::{Closure, ClosureKind};
pub use constructor::{Constructor, ConstructorKind, Destructor, DestructorKind};
pub use context::{ContextComponent, SymbolContext};
pub use descriptor::{Descriptor, DescriptorKind};
pub use enum_case::EnumCase;
pub use function::Function;
pub use macro_symbol::{MacroSymbol, MacroSymbolKind};
pub use metadata::{Metadata, MetadataKind};
pub use outlined::{Outlined, OutlinedKind};
pub use specialization::{
    FunctionSignatureParam, FunctionSignatureParamFlags, FunctionSignatureParamKind,
    Specialization, SpecializationKind,
};
pub use symbol::{
    AttributedSymbol, DefaultArgument, OutlinedSymbol, SpecializedSymbol, SuffixedSymbol, Symbol,
    SymbolAttribute, Variable,
};
pub use thunk::{
    AutoDiffThunk, AutoDiffThunkKind, DispatchKind, OtherThunkKind, ProtocolWitnessThunk,
    ReabstractionThunk, Thunk,
};
pub use types::{
    FunctionConvention, FunctionParam, FunctionType, GenericRequirement, GenericRequirementKind,
    GenericSignature, ImplFunctionType, ImplParam, ImplResult, NamedType, OpaqueSource,
    TupleElement, TypeKind, TypeRef,
};
pub use witness_table::{ProtocolConformance, ValueWitnessKind, WitnessTable, WitnessTableKind};
