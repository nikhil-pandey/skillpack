use std::path::{MAIN_SEPARATOR, Path};

/// Abbreviate home directory to ~
pub(crate) fn abbreviate_path(path: &str) -> String {
    let Some(home) = dirs::home_dir() else {
        return path.to_string();
    };
    let path_buf = Path::new(path);
    let Ok(stripped) = path_buf.strip_prefix(&home) else {
        return path.to_string();
    };
    if stripped.as_os_str().is_empty() {
        return "~".to_string();
    }
    format!("~{}{}", MAIN_SEPARATOR, stripped.display())
}

pub(crate) fn short_hash(hash: &str) -> String {
    let end = hash.len().min(8);
    hash[..end].to_string()
}

#[cfg(test)]
mod tests {
    use super::abbreviate_path;
    use std::path::MAIN_SEPARATOR;

    #[test]
    fn abbreviate_path_respects_segment_boundary() {
        let Some(home) = dirs::home_dir() else {
            return;
        };
        let home_str = home.to_string_lossy().to_string();
        let sibling = format!("{home_str}x");
        assert_eq!(abbreviate_path(&sibling), sibling);
    }

    #[test]
    fn abbreviate_path_abbreviates_home_and_child() {
        let Some(home) = dirs::home_dir() else {
            return;
        };
        let home_str = home.to_string_lossy().to_string();
        assert_eq!(abbreviate_path(&home_str), "~");

        let child_str = home.join("child").to_string_lossy().to_string();
        let expected = format!("~{}child", MAIN_SEPARATOR);
        assert_eq!(abbreviate_path(&child_str), expected);
    }
}
