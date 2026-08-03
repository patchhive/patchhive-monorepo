use std::{env, fs, path::PathBuf};

#[derive(Debug)]
struct ProductSource {
    key: String,
    module: String,
    order: usize,
    file_name: String,
}

fn manifest_value<'a>(source: &'a str, field: &str) -> Option<&'a str> {
    source.lines().find_map(|line| {
        let (key, value) = line.split_once('=')?;
        (key.trim() == field).then(|| value.trim().trim_matches('"'))
    })
}

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("manifest directory"));
    let registry_dir = manifest_dir.join("registry/products");
    println!("cargo:rerun-if-changed={}", registry_dir.display());

    let mut products = fs::read_dir(&registry_dir)
        .expect("read product registry")
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "toml"))
        .map(|entry| {
            let file_name = entry.file_name().to_string_lossy().into_owned();
            let source = fs::read_to_string(entry.path()).expect("read product manifest");
            let key = manifest_value(&source, "key")
                .expect("product manifest key")
                .to_string();
            let module_path =
                manifest_value(&source, "module_path").expect("product manifest module_path");
            let module = module_path
                .strip_prefix("crate::products::")
                .unwrap_or(module_path)
                .to_string();
            assert!(
                module
                    .chars()
                    .all(|ch| ch == '_' || ch.is_ascii_alphanumeric()),
                "product '{key}' has invalid Rust module identifier '{module}'"
            );
            let order = manifest_value(&source, "order")
                .expect("product manifest display order")
                .parse::<usize>()
                .expect("numeric product display order");
            ProductSource {
                key,
                module,
                order,
                file_name,
            }
        })
        .collect::<Vec<_>>();
    products.sort_by_key(|product| product.order);

    let manifest_sources = products
        .iter()
        .map(|product| {
            format!(
                "include_str!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/registry/products/{}\"))",
                product.file_name
            )
        })
        .collect::<Vec<_>>()
        .join(",\n    ");

    // HiveCore starts last because its background workers may immediately observe
    // or dispatch to every other enabled engine.
    products.sort_by_key(|product| (product.key == "hive-core", product.order));
    let wiring = products
        .iter()
        .map(|product| format!("({}, \"{}\")", product.module, product.key))
        .collect::<Vec<_>>()
        .join(",\n            ");

    let generated = format!(
        r#"
pub const PRODUCT_MANIFEST_SOURCES: &[&str] = &[
    {manifest_sources}
];

macro_rules! for_each_product {{
    ($callback:ident) => {{
        $callback!(
            {wiring}
        )
    }};
}}
"#
    );
    let output = PathBuf::from(env::var("OUT_DIR").expect("build output directory"))
        .join("product_inventory.rs");
    fs::write(output, generated).expect("write generated product inventory");
}
