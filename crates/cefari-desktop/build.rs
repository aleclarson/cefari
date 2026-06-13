fn main() {
    #[cfg(target_os = "macos")]
    {
        use std::{env, path::PathBuf, process::Command};

        let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR should be set by Cargo"));
        let object = out_dir.join("macos_cef_app_protocol.o");
        let compiler = env::var("CC").unwrap_or_else(|_| "cc".to_owned());
        let status = Command::new(&compiler)
            .args(["-fobjc-arc", "-c", "src/macos_cef_app_protocol.m", "-o"])
            .arg(&object)
            .status()
            .unwrap_or_else(|error| panic!("failed to run {compiler}: {error}"));

        assert!(
            status.success(),
            "{compiler} failed to compile src/macos_cef_app_protocol.m"
        );

        println!("cargo:rerun-if-changed=src/macos_cef_app_protocol.m");
        // Link the object directly; archived Objective-C category metadata can be skipped.
        println!("cargo:rustc-link-arg={}", object.display());
    }
}
