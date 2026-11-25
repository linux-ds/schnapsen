use std::{env, fs, path::PathBuf, process::Command};

fn main() {
    println!("cargo:rerun-if-changed=data/io.github.schnapsen.Schnapsen.gschema.xml");
    let output = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo must set OUT_DIR"));
    let status = Command::new("glib-compile-schemas")
        .arg("--strict")
        .arg("--targetdir")
        .arg(&output)
        .arg("data")
        .status()
        .expect("glib-compile-schemas is required to build Schnapsen");
    assert!(status.success(), "could not compile the Schnapsen schema");

    println!("cargo:rerun-if-changed=po/LINGUAS");
    let languages = fs::read_to_string("po/LINGUAS").expect("could not read po/LINGUAS");
    for language in languages.lines().flat_map(|line| {
        line.split('#')
            .next()
            .unwrap_or_default()
            .split_whitespace()
    }) {
        let catalog = format!("po/{language}.po");
        println!("cargo:rerun-if-changed={catalog}");
        let messages = output.join("locale").join(language).join("LC_MESSAGES");
        fs::create_dir_all(&messages).expect("could not create Cargo locale directory");
        let status = Command::new("msgfmt")
            .arg("--check")
            .arg("--output-file")
            .arg(messages.join("schnapsen.mo"))
            .arg(&catalog)
            .status()
            .expect("msgfmt is required to build Schnapsen's translations");
        assert!(status.success(), "could not compile {catalog}");
    }
}
