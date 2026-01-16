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

    // On MSVC, always use the release CRT to match Rust's linkage.
    // Debug builds otherwise use MSVCRTD which conflicts at link time.
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        cmake_config.profile("RelWithDebInfo");
    }

    let dst = cmake_config.build();

    println!("cargo:rustc-link-search=native={}/lib", dst.display());
    println!("cargo:rustc-link-lib=static=swift_demangle");

    println!("cargo:rerun-if-changed=swift-demangling/src/swift_demangle.cpp");
    println!("cargo:rerun-if-changed=swift-demangling/include/swift_demangle.h");
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
