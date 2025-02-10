use std::env;
use std::path::PathBuf;

fn main() {
    generate_bindings();
}

fn generate_bindings() {
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    let static_fns_path = out_path.join("static_fns.c");
    let bindings_path = out_path.join("bindings.rs");

    // Tell cargo to look for Babeltrace2.
    println!("cargo:rustc-link-lib=babeltrace2");

    // Generate bindings
    let bindings = bindgen::Builder::default()
        .header("src/bindings.h")
        .default_enum_style(bindgen::EnumVariation::NewType {
            is_bitfield: false,
            is_global: false,
        })
        .wrap_static_fns(true)
        .wrap_static_fns_path(&static_fns_path)
        .generate()
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    bindings
        .write_to_file(&bindings_path)
        .expect("Couldn't write bindings!");

    // Compile the static functions
    cc::Build::new()
        .file(static_fns_path)
        .include(env::var("CARGO_MANIFEST_DIR").unwrap())
        .compile("static_fns");
}
