use std::env;

fn main() {
    let dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let config = cbindgen::Config::from_file(format!("{dir}/cbindgen.toml")).unwrap();
    cbindgen::Builder::new()
        .with_crate(&dir)
        .with_config(config)
        .with_after_include(format!(
            "\nconstexpr uint16_t WB_MAX_PAYLOAD_LEN = {};",
            common::MAX_PAYLOAD_LEN
        ))
        .generate()
        .expect("cbindgen failed")
        .write_to_file(format!(
            "{dir}/../bridge/components/weatherboard_ffi/weatherboard.hpp"
        ));
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=cbindgen.toml");
    println!("cargo:rerun-if-changed=../common/src");
}
