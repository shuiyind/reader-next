use std::path::{Component, Path};

pub fn safe_path_component(value: &str, label: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() || value == "." || value == ".." {
        return Err(format!("{label} 非法"));
    }
    if value.chars().any(|ch| ch.is_control() || matches!(ch, '/' | '\\' | ':' | '\0')) {
        return Err(format!("{label} 非法"));
    }

    let path = Path::new(value);
    let mut components = path.components();
    let is_single_normal = matches!(components.next(), Some(Component::Normal(part)) if part.to_string_lossy().as_ref() == value)
        && components.next().is_none();
    if !is_single_normal {
        return Err(format!("{label} 非法"));
    }
    Ok(value.to_string())
}

pub fn safe_relative_parts(path: &str) -> Result<Vec<String>, String> {
    let mut parts = Vec::new();
    for raw in path.split(|ch| ch == '/' || ch == '\\') {
        if raw.is_empty() || raw == "." {
            continue;
        }
        parts.push(safe_path_component(raw, "路径")?);
    }
    Ok(parts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_traversal_and_absolute_components() {
        for value in ["..", "../x", "/tmp/x", r"..\x", r"C:\x", "a/b", "a\\b"] {
            assert!(safe_path_component(value, "文件名").is_err(), "{value}");
        }
    }

    #[test]
    fn accepts_single_component() {
        assert_eq!(safe_path_component("cover.png", "文件名").unwrap(), "cover.png");
    }
}
