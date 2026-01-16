use clap::Parser;
use swift_demangler::raw::{Context, FunctionInfo, Node, demangle};
use swift_demangler::{
    FunctionType, HasExtensionContext, HasFunctionSignature, HasGenericSignature, HasModule, Symbol,
};

fn print_signature(sig: &FunctionType, labels: &[Option<&str>], indent: &str) {
    let params = sig.parameters();
    println!("{indent}Parameters: {}", params.len());
    for (i, param) in params.iter().enumerate() {
        let label = labels
            .get(i)
            .copied()
            .flatten()
            .or(param.label)
            .unwrap_or("_");
        let variadic = if param.is_variadic { "..." } else { "" };
        println!(
            "{indent}  [{i}] {}: {}{} ({:?})",
            label, param.type_ref, variadic, param.type_ref
        );
    }
    if let Some(ret) = sig.return_type() {
        println!("{indent}Return Type: {} ({:?})", ret, ret);
    }
}

fn print_symbol(symbol: &Symbol, indent: &str) {
    let inner_indent = format!("{indent}  ");
    match symbol {
        Symbol::Function(func) => {
            println!("{indent}Type: Function");
            println!("{indent}Name: {}", func.name().unwrap_or("(unknown)"));
            println!("{indent}Full Name: {}", func.full_name());
            println!("{indent}Module: {}", func.module().unwrap_or("(unknown)"));
            println!("{indent}Is Method: {}", func.is_method());
            if func.is_method() {
                println!(
                    "{indent}Containing Type: {}",
                    func.containing_type().unwrap_or("(unknown)")
                );
            }
            println!("{indent}Is Static: {}", func.is_static());
            println!("{indent}Is Extension: {}", func.is_extension());
            if func.is_extension() {
                if let Some(ext_mod) = func.extension_module() {
                    println!("{indent}Extension Module: {}", ext_mod);
                }
            }
            println!("{indent}Async: {}", func.is_async());
            println!("{indent}Throws: {}", func.is_throwing());
            println!("{indent}Is Generic: {}", func.is_generic());
            let requirements = func.generic_requirements();
            if !requirements.is_empty() {
                println!("{indent}Generic Constraints:");
                for req in &requirements {
                    println!("{indent}  {}", req);
                }
            }
            if let Some(sig) = func.signature() {
                print_signature(&sig, &func.labels(), indent);
            }
        }
        Symbol::Constructor(ctor) => {
            println!("{indent}Type: Constructor");
            println!("{indent}Kind: {:?}", ctor.kind());
            println!(
                "{indent}Containing Type: {}",
                ctor.containing_type().unwrap_or("(unknown)")
            );
            println!("{indent}Module: {}", ctor.module().unwrap_or("(unknown)"));
            println!("{indent}Is Extension: {}", ctor.is_extension());
            if ctor.is_extension() {
                if let Some(ext_mod) = ctor.extension_module() {
                    println!("{indent}Extension Module: {}", ext_mod);
                }
                let ext_requirements = ctor.extension_generic_requirements();
                if !ext_requirements.is_empty() {
                    println!("{indent}Extension Generic Constraints:");
                    for req in &ext_requirements {
                        println!("{indent}  {}", req);
                    }
                }
            }
            println!("{indent}Async: {}", ctor.is_async());
            println!("{indent}Throws: {}", ctor.is_throwing());
            println!("{indent}Failable: {}", ctor.is_failable());
            println!("{indent}Is Generic: {}", ctor.is_generic());
            let requirements = ctor.generic_requirements();
            if !requirements.is_empty() {
                println!("{indent}Generic Constraints:");
                for req in &requirements {
                    println!("{indent}  {}", req);
                }
            }
            if let Some(sig) = ctor.signature() {
                print_signature(&sig, &ctor.labels(), indent);
            }
        }
        Symbol::Destructor(dtor) => {
            println!("{indent}Type: Destructor");
            println!("{indent}Kind: {:?}", dtor.kind());
            println!(
                "{indent}Containing Type: {}",
                dtor.containing_type().unwrap_or("(unknown)")
            );
            println!("{indent}Module: {}", dtor.module().unwrap_or("(unknown)"));
        }
        Symbol::EnumCase(ec) => {
            println!("{indent}Type: EnumCase");
            println!(
                "{indent}Case Name: {}",
                ec.case_name().unwrap_or("(unknown)")
            );
            println!(
                "{indent}Containing Type: {}",
                ec.containing_type().unwrap_or("(unknown)")
            );
            println!("{indent}Module: {}", ec.module().unwrap_or("(unknown)"));
            println!(
                "{indent}Has Associated Values: {}",
                ec.has_associated_values()
            );
            println!("{indent}Is Generic: {}", ec.is_generic());
            let requirements = ec.generic_requirements();
            if !requirements.is_empty() {
                println!("{indent}Generic Constraints:");
                for req in &requirements {
                    println!("{indent}  {}", req);
                }
            }
            let associated = ec.associated_values();
            if !associated.is_empty() {
                println!("{indent}Associated Values:");
                for (i, val) in associated.iter().enumerate() {
                    println!("{indent}  [{i}]: {}", val);
                }
            }
            if let Some(sig) = ec.signature() {
                print_signature(&sig, &[], indent);
            }
        }
        Symbol::Accessor(acc) => {
            println!("{indent}Type: Accessor");
            println!("{indent}Kind: {:?}", acc.kind());
            println!(
                "{indent}Property: {}",
                acc.property_name().unwrap_or("(unknown)")
            );
            println!("{indent}Module: {}", acc.module().unwrap_or("(unknown)"));
            if let Some(containing) = acc.containing_type() {
                println!("{indent}Containing Type: {}", containing);
            }
            println!("{indent}Is Static: {}", acc.is_static());
            println!("{indent}Is Extension: {}", acc.is_extension());
            if acc.is_extension() {
                if let Some(ext_mod) = acc.extension_module() {
                    println!("{indent}Extension Module: {}", ext_mod);
                }
            }
            println!("{indent}Is Subscript: {}", acc.is_subscript());
            println!("{indent}Is Mutating: {}", acc.is_mutating());
            println!("{indent}Is Generic: {}", acc.is_generic());
            let requirements = acc.generic_requirements();
            if !requirements.is_empty() {
                println!("{indent}Generic Constraints:");
                for req in &requirements {
                    println!("{indent}  {}", req);
                }
            }
            if let Some(prop_type) = acc.property_type() {
                println!("{indent}Property Type: {}", prop_type);
            }
        }
        Symbol::Variable(var) => {
            println!("{indent}Type: Variable");
            println!("{indent}Name: {}", var.name().unwrap_or("(unknown)"));
            println!("{indent}Module: {}", var.module().unwrap_or("(unknown)"));
            if let Some(var_type) = var.variable_type() {
                println!("{indent}Variable Type: {}", var_type);
            }
        }
        Symbol::Closure(closure) => {
            println!("{indent}Type: Closure");
            println!("{indent}Kind: {:?}", closure.kind());
            if let Some(idx) = closure.index() {
                println!("{indent}Index: {}", idx);
            }
            if let Some(discriminator) = closure.discriminator() {
                println!("{indent}Discriminator: {}", discriminator);
            }
            println!(
                "{indent}Module: {}",
                closure.module().unwrap_or("(unknown)")
            );
            if let Some(parent) = closure.parent_function() {
                println!("{indent}Parent Function: {}", parent);
            }
            println!("{indent}Async: {}", closure.is_async());
            println!("{indent}Throws: {}", closure.is_throwing());
            println!("{indent}Is Generic: {}", closure.is_generic());
            let requirements = closure.generic_requirements();
            if !requirements.is_empty() {
                println!("{indent}Generic Constraints:");
                for req in &requirements {
                    println!("{indent}  {}", req);
                }
            }
        }
        Symbol::Thunk(thunk) => {
            println!("{indent}Type: Thunk");
            println!("{indent}Kind: {}", thunk.kind_name());
            println!("{indent}Module: {}", thunk.module().unwrap_or("(unknown)"));
            match thunk {
                swift_demangler::Thunk::Reabstraction(t) => {
                    if let Some(source) = t.source() {
                        println!("{indent}From: {}", source);
                    }
                    if let Some(target) = t.target() {
                        println!("{indent}To: {}", target);
                    }
                    if let Some(sig) = t.generic_signature() {
                        println!("{indent}Generic Signature: {:?}", sig);
                    }
                }
                swift_demangler::Thunk::ProtocolWitness(t) => {
                    if let Some(conformance) = t.conformance() {
                        println!("{indent}Conformance:");
                        if let Some(conforming) = conformance.conforming_type() {
                            println!("{indent}  Conforming Type: {}", conforming);
                        }
                        if let Some(proto) = conformance.protocol() {
                            println!("{indent}  Protocol: {}", proto);
                        }
                        if let Some(module) = conformance.module() {
                            println!("{indent}  Module: {}", module);
                        }
                    }
                    if let Some(inner) = t.inner() {
                        println!("{indent}Inner Symbol:");
                        print_symbol(&inner, &inner_indent);
                    }
                }
                swift_demangler::Thunk::AutoDiff(t) => {
                    println!("{indent}AutoDiff Kind: {:?}", t.kind());
                    if let Some(func) = t.function() {
                        println!(
                            "{indent}Original Function: {}",
                            func.name().unwrap_or("(unknown)")
                        );
                    }
                    if let Some(params) = t.parameter_indices() {
                        println!("{indent}Parameter Indices: {}", params);
                    }
                    if let Some(results) = t.result_indices() {
                        println!("{indent}Result Indices: {}", results);
                    }
                }
                swift_demangler::Thunk::Dispatch { inner, kind, .. } => {
                    println!("{indent}Dispatch Kind: {:?}", kind);
                    println!("{indent}Inner Symbol:");
                    print_symbol(inner, &inner_indent);
                }
                swift_demangler::Thunk::PartialApply { inner, is_objc, .. } => {
                    if *is_objc {
                        println!("{indent}ObjC: true");
                    }
                    if let Some(inner) = inner {
                        println!("{indent}Inner Symbol:");
                        print_symbol(inner, &inner_indent);
                    }
                }
                swift_demangler::Thunk::Other { kind, inner, .. } => {
                    println!("{indent}Other Kind: {:?}", kind);
                    if let Some(inner) = inner {
                        println!("{indent}Inner Symbol:");
                        print_symbol(inner, &inner_indent);
                    }
                }
            }
        }
        Symbol::Specialization(spec) => {
            println!("{indent}Type: Specialization");
            println!("{indent}Kind: {:?}", spec.specialization.kind());
            if let Some(pass_id) = spec.specialization.pass_id() {
                println!("{indent}Pass ID: {}", pass_id);
            }
            let type_args = spec.specialization.type_arguments();
            if !type_args.is_empty() {
                println!("{indent}Type Arguments:");
                for (i, arg) in type_args.iter().enumerate() {
                    println!("{indent}  [{i}]: {}", arg);
                }
            }
            let func_params = spec.specialization.function_signature_params();
            if !func_params.is_empty() {
                println!("{indent}Function Signature Params:");
                for (i, param) in func_params.iter().enumerate() {
                    println!("{indent}  [{i}]: {}", param.kind().name());
                    let flags = param.flags();
                    if flags.any() {
                        println!("{indent}      Flags: {}", flags.names().join(", "));
                    }
                    for payload in param.payloads() {
                        // Demangle the payload if it's a mangled symbol
                        let display = demangle(payload).unwrap_or_else(|| payload.to_string());
                        println!("{indent}      Payload: {}", display);
                    }
                    let types = param.types();
                    for (j, t) in types.iter().enumerate() {
                        println!("{indent}      Type[{j}]: {}", t);
                    }
                }
            }
            println!("{indent}Inner Symbol:");
            print_symbol(&spec.inner, &inner_indent);
        }
        Symbol::WitnessTable(wt) => {
            println!("{indent}Type: WitnessTable");
            println!("{indent}Kind: {:?}", wt.kind());
            if let Some(vw_kind) = wt.value_witness_kind() {
                println!("{indent}Value Witness: {}", vw_kind.name());
            }
            // Associated type info (for AssociatedTypeWitnessTableAccessor)
            let assoc_path = wt.associated_type_path();
            if !assoc_path.is_empty() {
                println!("{indent}Associated Type: {}", assoc_path.join("."));
            }
            if let Some(assoc_proto) = wt.associated_type_protocol() {
                println!("{indent}Associated Type Protocol: {}", assoc_proto);
            }
            if let Some(conformance) = wt.conformance() {
                println!("{indent}Conformance:");
                if let Some(conforming) = conformance.conforming_type() {
                    println!("{indent}  Conforming Type: {}", conforming);
                }
                if let Some(proto) = conformance.protocol() {
                    println!("{indent}  Protocol: {}", proto);
                }
                if let Some(module) = conformance.module() {
                    println!("{indent}  Module: {}", module);
                }
            } else if let Some(conforming) = wt.conforming_type() {
                // For ValueWitnessTable and others without a ProtocolConformance
                println!("{indent}Conforming Type: {}", conforming);
            }
        }
        Symbol::Descriptor(desc) => {
            println!("{indent}Type: Descriptor");
            println!("{indent}Kind: {:?}", desc.kind());
            if let Some(conformance) = desc.conformance() {
                println!("{indent}Conformance:");
                if let Some(conforming) = conformance.conforming_type() {
                    println!("{indent}  Conforming Type: {}", conforming);
                }
                if let Some(proto) = conformance.protocol() {
                    println!("{indent}  Protocol: {}", proto);
                }
                if let Some(module) = conformance.module() {
                    println!("{indent}  Module: {}", module);
                }
            } else if let Some(func) = desc.described_function() {
                // Method descriptor - show full function information
                println!("{indent}Described Method:");
                println!("{indent}  Name: {}", func.name().unwrap_or("(unknown)"));
                println!("{indent}  Full Name: {}", func.full_name());
                if let Some(containing) = func.containing_type() {
                    println!("{indent}  Containing Type: {}", containing);
                }
                println!("{indent}  Is Method: {}", func.is_method());
                println!("{indent}  Async: {}", func.is_async());
                println!("{indent}  Throws: {}", func.is_throwing());
                println!("{indent}  Is Generic: {}", func.is_generic());
                let requirements = func.generic_requirements();
                if !requirements.is_empty() {
                    println!("{indent}  Generic Constraints:");
                    for req in &requirements {
                        println!("{indent}    {}", req);
                    }
                }
                if let Some(sig) = func.signature() {
                    print_signature(&sig, &func.labels(), &format!("{indent}  "));
                }
                if let Some(module) = func.module() {
                    println!("{indent}  Module: {}", module);
                }
            } else if let Some(described) = desc.described_type() {
                println!("{indent}Described Type: {}", described);
            }
            if desc.conformance().is_none() && desc.described_function().is_none() {
                if let Some(module) = desc.module() {
                    println!("{indent}Module: {}", module);
                }
            }
        }
        Symbol::Metadata(meta) => {
            println!("{indent}Type: Metadata");
            println!("{indent}Kind: {:?}", meta.kind());
            if let Some(meta_type) = meta.metadata_type() {
                println!("{indent}Metadata For: {}", meta_type);
            }
            if let Some(inner) = meta.inner() {
                println!("{indent}Inner:");
                print_symbol(inner, &inner_indent);
            }
            if meta.is_accessor() {
                println!("{indent}Is Accessor: true");
            }
            if meta.is_cache() {
                println!("{indent}Is Cache: true");
            }
        }
        Symbol::Attributed(attr) => {
            println!("{indent}Type: Attributed");
            println!("{indent}Attribute: {}", attr.attribute.name());
            println!("{indent}Inner Symbol:");
            print_symbol(&attr.inner, &inner_indent);
        }
        Symbol::DefaultArgument(default_arg) => {
            println!("{indent}Type: DefaultArgument");
            if let Some(idx) = default_arg.index() {
                println!("{indent}Argument Index: {}", idx);
            }
            if let Some(func) = default_arg.function() {
                println!("{indent}Function: {}", func.full_name());
                println!("{indent}Module: {}", func.module().unwrap_or("(unknown)"));
            }
        }
        Symbol::Type(type_ref) => {
            println!("{indent}Type: Type Symbol");
            println!("{indent}Display: {}", type_ref);
            println!("{indent}Kind: {:?}", type_ref.kind());
        }
        Symbol::Outlined(outlined_sym) => {
            println!("{indent}Type: Outlined");
            println!("{indent}Kind: {:?}", outlined_sym.outlined.kind());
            if let Some(idx) = outlined_sym.outlined.index() {
                println!("{indent}Index: {}", idx);
            }
            if let Some(module) = outlined_sym.outlined.module() {
                println!("{indent}Module: {}", module);
            }
            println!("{indent}Context:");
            print_symbol(&outlined_sym.context, &inner_indent);
        }
        Symbol::Async(async_sym) => {
            println!("{indent}Type: Async");
            println!("{indent}Kind: {:?}", async_sym.kind());
            if let Some(idx) = async_sym.partial_index() {
                println!("{indent}Partial Index: {}", idx);
            }
            if let Some(inner) = async_sym.inner() {
                println!("{indent}Inner:");
                print_symbol(inner, &inner_indent);
            }
        }
        Symbol::Macro(macro_sym) => {
            println!("{indent}Type: Macro");
            println!("{indent}Kind: {:?}", macro_sym.kind());
            if let Some(name) = macro_sym.name() {
                println!("{indent}Name: {}", name);
            }
            if let Some(num) = macro_sym.expansion_number() {
                println!("{indent}Expansion #: {}", num);
            }
            if let Some(module) = macro_sym.module() {
                println!("{indent}Module: {}", module);
            }
            if let Some(file) = macro_sym.file() {
                println!("{indent}File: {}", file);
            }
            if let Some(line) = macro_sym.line() {
                println!("{indent}Line: {}", line);
            }
            if let Some(column) = macro_sym.column() {
                println!("{indent}Column: {}", column);
            }
        }
        Symbol::AutoDiff(autodiff) => {
            println!("{indent}Type: AutoDiff");
            println!("{indent}Kind: {:?}", autodiff.kind());
            if let Some(func) = autodiff.inner_function() {
                println!("{indent}Inner Function: {}", func.full_name());
            }
            if let Some(module) = autodiff.module() {
                println!("{indent}Module: {}", module);
            }
        }
        Symbol::Identifier(node) => {
            println!("{indent}Type: Identifier");
            if let Some(name) = node.text() {
                println!("{indent}Name: {}", name);
            }
        }
        Symbol::Other(node) => {
            println!("{indent}Type: Other");
            println!("{indent}Node Kind: {:?}", node.kind());
            println!("{indent}Display: {}", node);
        }
        Symbol::Suffixed(suffixed) => {
            println!("{indent}Type: Suffixed");
            println!("{indent}Suffix: {}", suffixed.suffix);
            println!("{indent}Inner:");
            print_symbol(&suffixed.inner, &inner_indent);
        }
    }
}

#[derive(Parser)]
#[command(name = "swift-demangler")]
#[command(about = "Swift Symbol Demangler")]
struct Args {
    /// Print the full node tree
    #[arg(short = 't', long = "tree")]
    tree: bool,

    /// Show function details (async, throws, params) using raw FFI
    #[arg(short = 'f', long = "function")]
    function: bool,

    /// Show structured high-level symbol information
    #[arg(short = 's', long = "structured")]
    structured: bool,

    /// Print the debug representation of the parsed symbol
    #[arg(short = 'd', long = "debug")]
    debug: bool,

    /// The mangled Swift symbol
    symbol: String,
}

fn main() {
    let args = Args::parse();

    // Simple demangling
    let Some(demangled) = demangle(&args.symbol) else {
        eprintln!("Failed to demangle: {}", args.symbol);
        std::process::exit(1);
    };
    println!("Demangled: {demangled}");

    // Node tree view
    if args.tree {
        let ctx = Context::new();
        if let Some(root) = Node::parse(&ctx, &args.symbol) {
            println!("\nNode tree:\n{root:#?}");
        }
    }

    // Function info (raw FFI)
    if args.function {
        if let Some(info) = FunctionInfo::parse(&args.symbol) {
            println!("\nFunction Info:");
            println!("  Module: {}", info.module_name().unwrap_or("(unknown)"));
            println!("  Name: {}", info.function_name().unwrap_or("(unknown)"));
            println!("  Async: {}", if info.is_async() { "yes" } else { "no" });
            println!(
                "  Throws: {}",
                if info.is_throwing() { "yes" } else { "no" }
            );
            println!(
                "  Typed Throws: {}",
                if info.has_typed_throws() { "yes" } else { "no" }
            );
            println!("  Parameters: {}", info.parameter_types().len());
            for (i, param) in info.parameter_types().iter().enumerate() {
                println!("    [{i}]: {param}");
            }
            println!("  Return Type: {}", info.return_type().unwrap_or("(none)"));
        } else {
            println!("\n(Not a function symbol)");
        }
    }

    // Structured high-level view
    if args.structured {
        let ctx = Context::new();
        if let Some(symbol) = Symbol::parse(&ctx, &args.symbol) {
            println!("\nStructured Symbol:");
            print_symbol(&symbol, "  ");
        } else {
            println!("\n(Failed to parse symbol)");
        }
    }

    // Debug representation
    if args.debug {
        let ctx = Context::new();
        if let Some(symbol) = Symbol::parse(&ctx, &args.symbol) {
            println!("\n{:#?}", symbol);
        } else {
            println!("\n(Failed to parse symbol)");
        }
    }
}
