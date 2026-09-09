fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    #[cfg(feature = "open303")]
    build_open303();
}

#[cfg(feature = "open303")]
fn build_open303() {
    use std::{env, fs, path::PathBuf};
    let directory = "vendor/open303/src";
    println!("cargo:rerun-if-changed={directory}");
    println!("cargo:rerun-if-changed=native/open303");
    println!("cargo:rerun-if-changed=tests/open303_native.cpp");
    let mut sources: Vec<_> = fs::read_dir(directory)
        .expect("Open303 source directory")
        .map(|entry| entry.expect("source entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "cpp"))
        .collect();
    sources.sort();
    let mut build = cc::Build::new();
    build
        .cpp(true)
        .std("c++17")
        .include(directory)
        .include("native/open303")
        .flag_if_supported("-Wno-unused-parameter")
        .flag_if_supported("-Werror")
        .files(sources)
        .file("native/open303/bridge.cpp");
    build.compile("moj_open303");
    // Native regressions also intercept C++ allocation, which Rust's allocator cannot see.
    // Build with the same compiler and library. No target executable is run by the build script.
    let out = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR"));
    let test = out.join("open303-native-test");
    let status = build
        .get_compiler()
        .to_command()
        .arg("-std=c++17")
        .arg("-UNDEBUG")
        .arg("-Ivendor/open303/src")
        .arg("-Inative/open303")
        .arg("tests/open303_native.cpp")
        .arg(out.join("libmoj_open303.a"))
        .arg("-o")
        .arg(&test)
        .status()
        .expect("native Open303 test compiler");
    assert!(status.success(), "native Open303 tests failed to build");
    println!("cargo:rustc-env=OPEN303_NATIVE_TEST={}", test.display());
}
