fn main() {
    #[cfg(windows)]
    {
        let mut resource = tauri_winres::WindowsResource::new();
        resource.set_icon("../desktop/src-tauri/icons/icon.ico");
        resource.set("FileDescription", "PLC Lab");
        resource.set("ProductName", "PLC Lab");
        if let Err(error) = resource.compile() {
            println!("cargo:warning=launcher icon not embedded: {error}");
        }
    }
}
