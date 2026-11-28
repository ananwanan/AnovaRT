// SPDX-License-Identifier: Apache-2.0

fn main() {
    let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../bsp/threebluewhale-f103ze/link.x");
    println!("cargo:rerun-if-changed={}", script.display());
    if std::env::var("TARGET").as_deref() == Ok("thumbv7m-none-eabi") {
        println!("cargo:rerun-if-env-changed=ANOVART_HEAP_BYTES");
        let pool = if std::env::var_os("CARGO_FEATURE_M3").is_some() {
            2048
        } else {
            0
        };
        let heap = if std::env::var_os("CARGO_FEATURE_HEAP_TLSF").is_some() {
            let bytes = std::env::var("ANOVART_HEAP_BYTES")
                .map_or(Ok(12288), |value| value.parse::<usize>())
                .expect("ANOVART_HEAP_BYTES must be a byte count");
            assert!(
                (4096..=65536).contains(&bytes) && bytes.is_multiple_of(8),
                "ANOVART_HEAP_BYTES must be 4096..=65536 and 8-byte aligned"
            );
            bytes
        } else {
            0
        };
        println!("cargo:rustc-link-arg-bin=anova-firmware=--defsym=__m3_pool_bytes={pool}");
        println!("cargo:rustc-link-arg-bin=anova-firmware=--defsym=__m3_heap_bytes={heap}");
        println!(
            "cargo:rustc-link-arg-bin=anova-firmware=-T{}",
            script.display()
        );
    }
}
