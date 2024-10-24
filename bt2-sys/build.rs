use std::env;
use std::path::PathBuf;

fn main() {
    generate_bindings();

    compile_sink_plugin();
}

fn compile_sink_plugin() {
    cc::Build::new().file("src-c/graph.c").compile("bt2-graph");

    println!("cargo:rerun-if-changed=src-c/graph.h");
    println!("cargo:rerun-if-changed=src-c/graph.c");
}

fn generate_bindings() {
    // Tell cargo to look for Babeltrace2.
    println!("cargo:rustc-link-lib=babeltrace2");

    // Generate bindings
    let bindings = bindgen::Builder::default()
        .header("src/bindings.h")
        .header("src-c/graph.h")
        .default_enum_style(bindgen::EnumVariation::NewType {
            is_bitfield: false,
            is_global: false,
        })
        .generate()
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
