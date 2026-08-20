// Build script for Slint UI
fn main() {
    slint_build::compile("ui/main_window.slint").unwrap();

    println!("cargo:rerun-if-changed=icons/GcodeFrameGen.ico");
    println!("cargo:rerun-if-changed=icons/app.rc");

    // Embed the Windows icon in the executable as well as setting it on the
    // runtime window. This makes the icon visible in File Explorer and when a
    // release build is launched directly.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let output = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("app-icon.o");
        let status = std::process::Command::new("windres")
            .args(["--input", "icons/app.rc", "--output"])
            .arg(&output)
            .args(["--output-format", "coff"])
            .status()
            .expect("windres is required to embed the Windows application icon");
        assert!(status.success(), "windres could not compile icons/app.rc");
        println!("cargo:rustc-link-arg={}", output.display());
    }
}
