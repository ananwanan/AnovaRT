// SPDX-License-Identifier: Apache-2.0

fn main() {
    let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../bsp/threebluewhale-f103ze/link.x");
    println!("cargo:rerun-if-changed={}", script.display());
    if std::env::var("TARGET").as_deref() == Ok("thumbv7m-none-eabi") {
        println!(
            "cargo:rustc-link-arg-bin=anova-firmware=-T{}",
            script.display()
        );
    }
}
