pub const PATCHHIVE_URL: &str = "https://github.com/patchhive";

pub fn product_signature(product: &str) -> String {
    format!("*{} by [PatchHive]({PATCHHIVE_URL})*", product.trim())
}

pub fn append_product_signature(markdown: &str, product: &str) -> String {
    let body = markdown.trim();
    let signature = product_signature(product);
    if body.ends_with(&signature) {
        body.to_string()
    } else {
        format!("{body}\n\n{signature}")
    }
}

#[cfg(test)]
mod tests {
    use super::{append_product_signature, product_signature};

    #[test]
    fn signature_links_the_product_to_patchhive() {
        assert_eq!(
            product_signature("MergeKeeper"),
            "*MergeKeeper by [PatchHive](https://github.com/patchhive)*"
        );
    }

    #[test]
    fn append_is_idempotent_for_an_existing_signature() {
        let once = append_product_signature("Report body", "ReviewBee");
        assert_eq!(append_product_signature(&once, "ReviewBee"), once);
    }
}
