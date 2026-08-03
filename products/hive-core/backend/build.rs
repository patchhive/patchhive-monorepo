use std::{env, fs, path::PathBuf};

fn manifest_value<'a>(source: &'a str, field: &str) -> Option<&'a str> {
    source.lines().find_map(|line| {
        let (key, value) = line.split_once('=')?;
        (key.trim() == field).then(|| value.trim().trim_matches('"'))
    })
}

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("manifest directory"));
    let registry_dir = manifest_dir.join("../../../services/patchhive-backend/registry/products");
    println!("cargo:rerun-if-changed={}", registry_dir.display());

    let mut products = fs::read_dir(&registry_dir)
        .expect("read canonical product registry")
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "toml"))
        .map(|entry| {
            let file_name = entry.file_name().to_string_lossy().into_owned();
            let source = fs::read_to_string(entry.path()).expect("read product manifest");
            let order = manifest_value(&source, "order")
                .expect("product manifest display order")
                .parse::<usize>()
                .expect("numeric product display order");
            (order, file_name)
        })
        .collect::<Vec<_>>();
    products.sort_by_key(|(order, _)| *order);

    let sources = products
        .iter()
        .map(|(_, file_name)| {
            format!(
                "include_str!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/../../../services/patchhive-backend/registry/products/{file_name}\"))"
            )
        })
        .collect::<Vec<_>>()
        .join(",\n    ");
    let generated = format!("pub const PRODUCT_MANIFESTS: &[&str] = &[\n    {sources}\n];\n");
    let output = PathBuf::from(env::var("OUT_DIR").expect("build output directory"))
        .join("product_manifests.rs");
    fs::write(output, generated).expect("write generated HiveCore product inventory");
}
