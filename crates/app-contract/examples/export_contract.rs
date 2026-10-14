fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../packages/ipc/src/generated.ts");
    std::fs::create_dir_all(path.parent().ok_or("Missing contract output directory")?)?;
    std::fs::write(&path, app_contract::typescript_contract())?;
    println!("Generated {}", path.display());
    Ok(())
}
