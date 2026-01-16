//! Snapshot tests for symbol parsing.
//!
//! This test reads all symbols from manglings.txt and creates a snapshot
//! of their Debug representation to catch regressions.

use swift_demangler::Symbol;
use swift_demangler::raw::{Context, demangle};

/// Read manglings.txt and extract the mangled symbols (before " --->")
fn read_manglings() -> Vec<String> {
    let content = include_str!("../swift-demangling/vendor/tests/manglings.txt");
    content
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with("//"))
        .filter_map(|line| line.split(" --->").next().map(|s| s.trim().to_string()))
        .filter(|s| !s.is_empty())
        .collect()
}

#[test]
fn test_all_manglings_snapshot() {
    let manglings = read_manglings();
    let ctx = Context::new();

    let mut output = String::new();
    for mangled in &manglings {
        output.push_str("--------\n\n");
        output.push_str(&format!("  Mangled: {mangled}\n"));

        // Include the demangled string
        if let Some(demangled) = demangle(mangled) {
            output.push_str("Demangled: ");
            output.push_str(&demangled);
            output.push('\n');
        }

        if let Some(symbol) = Symbol::parse(&ctx, mangled) {
            output.push_str(&format!("\n{symbol:#?}"));
        } else {
            output.push_str("\n(failed to parse)");
        }
        output.push_str("\n\n");
    }

    insta::assert_snapshot!(output);
}
