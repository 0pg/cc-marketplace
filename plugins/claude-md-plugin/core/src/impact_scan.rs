use regex::Regex;
use std::path::Path;
use walkdir::WalkDir;

pub fn scan(root: &Path, target: &str) -> std::io::Result<Vec<String>> {
    let dev = std::fs::read_to_string(root.join(target).join("DEVELOPERS.md"))?;
    let names = extract_names(&dev);
    if names.is_empty() {
        return Ok(vec![]);
    }

    let mut consumers = vec![];
    for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
        if entry.file_name() != "DEVELOPERS.md" {
            continue;
        }
        let rel = entry
            .path()
            .parent()
            .unwrap()
            .strip_prefix(root)
            .unwrap_or(Path::new(""))
            .to_string_lossy()
            .into_owned();
        if rel == target || rel.is_empty() {
            continue;
        }
        let body = std::fs::read_to_string(entry.path()).unwrap_or_default();
        if names.iter().any(|n| body.contains(n)) {
            consumers.push(rel);
        }
    }
    consumers.sort();
    Ok(consumers)
}

fn extract_names(doc: &str) -> Vec<String> {
    let re = Regex::new(r"(?m)^\s*(?:pub\s+)?(?:struct|enum|type|trait)\s+([A-Z]\w+)").unwrap();
    let mut names = vec![];
    let mut in_section = false;
    for line in doc.lines() {
        if line.starts_with("## ") {
            in_section = line.trim_end() == "## Data Schemas";
            continue;
        }
        if in_section {
            for c in re.captures_iter(line) {
                names.push(c[1].to_string());
            }
        }
    }
    names
}
