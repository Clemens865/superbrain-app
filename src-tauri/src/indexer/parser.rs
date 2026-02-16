//! File content parser for SuperBrain
//!
//! Extracts text content from supported file types.

use std::path::Path;

/// Supported file extensions
const SUPPORTED_EXTENSIONS: &[&str] = &[
    "md", "txt", "rs", "ts", "tsx", "js", "jsx", "py", "json", "toml", "yaml", "yml", "html",
    "css", "sh", "bash", "zsh", "fish", "swift", "go", "java", "c", "cpp", "h", "hpp", "rb",
    "lua", "sql", "xml", "csv", "log", "conf", "cfg", "ini", "env",
];

/// Check if a file extension is supported for indexing
pub fn is_supported(ext: &str) -> bool {
    SUPPORTED_EXTENSIONS.contains(&ext.to_lowercase().as_str())
}

/// Parse a file and extract its text content
pub fn parse_file(path: &Path) -> Result<String, String> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    if !is_supported(&ext) {
        return Err(format!("Unsupported file type: {}", ext));
    }

    // Read file content
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read {:?}: {}", path, e))?;

    // Strip content based on file type
    match ext.as_str() {
        "json" => parse_json(&content),
        "html" | "xml" => parse_markup(&content),
        _ => Ok(clean_text(&content)),
    }
}

/// Clean raw text content
fn clean_text(content: &str) -> String {
    // Remove excessive whitespace and empty lines
    content
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Parse JSON and extract meaningful text values
fn parse_json(content: &str) -> Result<String, String> {
    // For JSON, we extract string values that likely contain meaningful text
    // Simple approach: just return the raw content cleaned up
    Ok(clean_text(content))
}

/// Parse HTML/XML and strip tags
fn parse_markup(content: &str) -> Result<String, String> {
    // Simple tag stripping
    let mut result = String::with_capacity(content.len());
    let mut in_tag = false;
    let mut in_script = false;

    for ch in content.chars() {
        match ch {
            '<' => {
                in_tag = true;
                // Check if we're entering a script or style block
                let rest = &content[content.find('<').unwrap_or(0)..];
                if rest.starts_with("<script") || rest.starts_with("<style") {
                    in_script = true;
                }
                if rest.starts_with("</script") || rest.starts_with("</style") {
                    in_script = false;
                }
            }
            '>' => {
                in_tag = false;
            }
            _ => {
                if !in_tag && !in_script {
                    result.push(ch);
                }
            }
        }
    }

    Ok(clean_text(&result))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_supported_extensions() {
        assert!(is_supported("rs"));
        assert!(is_supported("ts"));
        assert!(is_supported("py"));
        assert!(is_supported("md"));
        assert!(!is_supported("exe"));
        assert!(!is_supported("png"));
        assert!(!is_supported("pdf"));
    }

    #[test]
    fn test_clean_text() {
        let input = "  hello  \n\n\n  world  \n  ";
        let result = clean_text(input);
        assert_eq!(result, "hello\nworld");
    }

    #[test]
    fn test_parse_markup() {
        let html = "<p>Hello <b>world</b></p>";
        let result = parse_markup(html).unwrap();
        assert!(result.contains("Hello"));
        assert!(result.contains("world"));
        assert!(!result.contains("<p>"));
    }
}
