//! This build script copies the `memory.x` file from the crate root into
//! a directory where the linker can always find it at build time.
//! For many projects this is optional, as the linker always searches the
//! project root directory -- wherever `Cargo.toml` is. However, if you
//! are using a workspace or have a more complicated build setup, this
//! build script becomes required. Additionally, by requesting that
//! Cargo re-run the build script whenever `memory.x` is changed,
//! updating `memory.x` ensures a rebuild of the application with the
//! new memory settings.

use std::env;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

fn hex_to_bytes(s: &str) -> Option<Vec<u8>> {
    if s.len() % 2 == 0 {
        (0..s.len())
            .step_by(2)
            .map(|i| s.get(i..i + 2).and_then(|sub| u8::from_str_radix(sub, 16).ok()))
            .collect()
    } else {
        None
    }
}

/// Read and parse LoRaWAN keys as HEX strings from an environment variable
fn parse_lorawan_id(val: Option<&str>, var: &str, len: usize) -> Option<String> {
    if let Some(s) = val {
        let l = s.len();
        // Allow empty keys
        if l == 0 {
            return None;
        }
        if l % 2 == 1 || l != 2 * len {
            panic!(
                "Environment variable {} has invalid length: {}, expecting: {}",
                &var,
                l,
                2 * len
            );
        }
        if let Some(v) = hex_to_bytes(s) {
            return Some(format!("Some({:?})", v));
        } else {
            panic!(
                "Unable to parse {} from environment, make sure it's a valid hex string with length {}",
                &var,
                2 * len
            );
        }
    }
    None
}

fn main() {
    let out = &PathBuf::from(env::var_os("OUT_DIR").unwrap());

    // Generate LoRaWAN eui and key overrides from environment variables
    {
        let path = &out.join("lorawan_keys.rs");
        let mut file = BufWriter::new(File::create(path).unwrap());

        // TODO: Figure out how to not generate this file every time...
        write!(
            &mut file,
            "{}",
            format_args!(
                "\
            // Generated by build.rs\n\
            const DEVEUI: Option<[u8; 8]> = {};\n\
            const APPEUI: Option<[u8; 8]> = {};\n\
            const APPKEY: Option<[u8; 16]> = {};\n",
                parse_lorawan_id(option_env!("LORA_DEVEUI"), "LORA_DEVEUI", 8).unwrap_or("None".to_string()),
                parse_lorawan_id(option_env!("LORA_APPEUI"), "LORA_APPEUI", 8).unwrap_or("None".to_string()),
                parse_lorawan_id(option_env!("LORA_APPKEY"), "LORA_APPKEY", 16).unwrap_or("None".to_string()),
            )
        )
        .unwrap();
    }

    // Put linker configuration in our output directory and ensure it's
    // on the linker search path.
    if cfg!(feature = "link-to-ram") {
        File::create(out.join("link_ram.x"))
            .unwrap()
            .write_all(include_bytes!("../link_ram_cortex_m.x"))
            .unwrap();
        println!("cargo:rustc-link-search={}", out.display());

        println!("cargo:rustc-link-arg-bins=-Tlink_ram.x");
        println!("cargo:rerun-if-changed=link_ram.x");
    } else {
        File::create(out.join("memory.x"))
            .unwrap()
            .write_all(include_bytes!("memory.x"))
            .unwrap();
        println!("cargo:rustc-link-search={}", out.display());

        println!("cargo:rustc-link-arg-bins=-Tlink.x");
        println!("cargo:rerun-if-changed=link.x");
    }

    println!("cargo:rustc-link-arg-bins=--nmagic");
    println!("cargo:rustc-link-arg-bins=-Tdefmt.x");
}
