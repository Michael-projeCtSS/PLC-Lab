fn main() {
    println!("cargo:rerun-if-changed=launcher.manifest");
    println!("cargo:rerun-if-changed=../desktop/src-tauri/icons/icon.ico");
    #[cfg(windows)]
    {
        let mut resource = tauri_winres::WindowsResource::new();
        resource.set_icon("../desktop/src-tauri/icons/icon.ico");
        resource.set("FileDescription", "PLC Lab");
        resource.set("ProductName", "PLC Lab");
        resource.set_manifest(include_str!("launcher.manifest"));
        if let Err(error) = resource.compile() {
            println!("cargo:warning=launcher resources not embedded: {error}");
        }
    }
}
