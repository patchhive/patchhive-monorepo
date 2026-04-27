// utils.rs - Utility functions for VulnTriage

pub fn location_label(path: &str, start_line: u32) -> String {
    if path.trim().is_empty() {
        return "unknown location".into();
    }
    if start_line > 0 {
        format!("{path}:{start_line}")
    } else {
        path.into()
    }
}

pub fn push_evidence(items: &mut Vec<String>, value: String) {
    if value.trim().is_empty() || items.iter().any(|item| item == &value) {
        return;
    }
    if items.len() < 8 {
        items.push(value);
    }
}

pub fn valid_repo(repo: &str) -> bool {
    let mut parts = repo.split('/');
    matches!(
        (parts.next(), parts.next(), parts.next()),
        (Some(owner), Some(name), None) if !owner.trim().is_empty() && !name.trim().is_empty()
    )
}
