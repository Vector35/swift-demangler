fn main() {
    // Check for pre-built library first
    if let Ok(lib_dir) = std::env::var("SWIFT_DEMANGLE_DIR") {
        println!("cargo:rustc-link-search=native={lib_dir}");
        println!("cargo:rustc-link-lib=static=swift_demangle");
        link_cpp_stdlib();
        return;
    }

    // Otherwise, build from source if bundled feature is enabled
    #[cfg(feature = "bundled")]
    {
        build_bundled();
    }

    #[cfg(not(feature = "bundled"))]
    {
        panic!(
            "swift-demangle requires either:\n\
             1. Set SWIFT_DEMANGLE_DIR to a directory containing the pre-built libraries, or\n\
             2. Enable the 'bundled' feature to build from source (default)"
        );
    }
}

#[cfg(feature = "bundled")]
fn build_bundled() {
    use std::path::PathBuf;
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let swift_demangling_dir = manifest_dir.join("swift-demangling");

    let mut cmake_config = cmake::Config::new(&swift_demangling_dir);
    cmake_config.define("BUILD_CLI", "OFF");

    // Unoptimized, NodePrinter overflows a 512 KiB stack on real symbols.
    // MSVC debug builds would also use the debug CRT, which conflicts with
    // the release CRT that Rust uses.
    cmake_config.profile("RelWithDebInfo");

    // On MSVC the cmake crate replaces these flags with its own to pass the
    // CRT choice, dropping the optimization flags. Set them ourselves.
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        let static_crt = std::env::var("CARGO_CFG_TARGET_FEATURE")
            .is_ok_and(|features| features.split(',').any(|f| f == "crt-static"));
        let crt = if static_crt { "/MT" } else { "/MD" };
        cmake_config.define(
            "CMAKE_CXX_FLAGS_RELWITHDEBINFO",
            format!("{crt} /O2 /Ob1 /DNDEBUG"),
        );
    }

    let dst = cmake_config.build();

    println!("cargo:rustc-link-search=native={}/lib", dst.display());
    println!("cargo:rustc-link-lib=static=swift_demangle");

    // Cargo checks directories recursively.
    println!("cargo:rerun-if-changed=swift-demangling/src");
    println!("cargo:rerun-if-changed=swift-demangling/include");
    println!("cargo:rerun-if-changed=swift-demangling/vendor/include");
    println!("cargo:rerun-if-changed=swift-demangling/vendor/lib");
    println!("cargo:rerun-if-changed=swift-demangling/CMakeLists.txt");

    link_cpp_stdlib();
}

fn link_cpp_stdlib() {
    let target = std::env::var("TARGET").unwrap();
    if target.contains("apple") {
        println!("cargo:rustc-link-lib=c++");
    } else if target.contains("linux") {
        println!("cargo:rustc-link-lib=stdc++");
    }
}
