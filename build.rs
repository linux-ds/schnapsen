use std::{env, process::Command};

fn main() {
    println!("cargo:rerun-if-changed=data/io.github.schnapsen.Schnapsen.gschema.xml");
    let output = env::var_os("OUT_DIR").expect("Cargo must set OUT_DIR");
    let status = Command::new("glib-compile-schemas")
        .arg("--strict")
        .arg("--targetdir")
        .arg(output)
        .arg("data")
        .status()
        .expect("glib-compile-schemas is required to build Schnapsen");
    assert!(status.success(), "could not compile the Schnapsen schema");
}
