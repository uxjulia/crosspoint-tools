fn main() {
    #[cfg(target_os = "macos")]
    {
        println!("cargo:rustc-link-lib=framework=ServiceManagement");
    }
    tauri_build::build()
}
