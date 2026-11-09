//! Compila el shim Objective-C de TLS (Network.framework) **solo en macOS**.
//!
//! Windows y Linux no pasan por acá: sus implementaciones son Rust puro
//! (Schannel / OpenSSL). El shim existe únicamente porque Network.framework es
//! asíncrona y la ABI de Falcato es síncrona (ver `tls_network_macos.m`).

fn main() {
    let objetivo = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if objetivo != "macos" {
        return;
    }

    println!("cargo:rerun-if-changed=tls_network_macos.m");

    cc::Build::new()
        .file("tls_network_macos.m")
        .flag("-fobjc-arc")
        .flag("-fblocks")
        .warnings(true)
        .compile("falcato_tls_network");

    println!("cargo:rustc-link-lib=framework=Foundation");
    println!("cargo:rustc-link-lib=framework=Network");
    println!("cargo:rustc-link-lib=framework=Security");
}
