use std::env;
use std::path::PathBuf;

fn main() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

    let config = cbindgen::Config {
        language: cbindgen::Language::C,
        documentation: true,
        header: Some(
            r#"
#ifndef GOOSE_FFI_H
#define GOOSE_FFI_H

/* Goose FFI - C interface for the Goose AI agent framework */
"#
            .trim_start()
            .to_string(),
        ),
        trailer: Some("#endif // GOOSE_FFI_H".to_string()),
        includes: vec![],
        sys_includes: vec!["stdint.h".to_string(), "stdbool.h".to_string()],
        export: cbindgen::ExportConfig {
            prefix: Some("goose_".to_string()),
            ..Default::default()
        },
        documentation_style: cbindgen::DocumentationStyle::C,
        enumeration: cbindgen::EnumConfig {
            prefix_with_name: true,
            derive_helper_methods: true,
            ..Default::default()
        },
        ..Default::default()
    };

    let bindings = cbindgen::Builder::new()
        .with_crate(&crate_dir)
        .with_config(config)
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(&crate_dir).join("include");
    std::fs::create_dir_all(&out_path).expect("Failed to create include directory");
    bindings.write_to_file(out_path.join("goose_ffi.h"));

    println!("cargo:rerun-if-changed=src/lib.rs");
    println!("cargo:rerun-if-changed=build.rs");
}
