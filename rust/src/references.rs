//! Load and append reference layouts from `config/references.toml`.
//!
//! Port of `shirolyzer/utils/references.py`.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};

use crate::paths;

#[derive(Clone, Debug)]
pub struct Reference {
    pub name: String,
    pub layout: String,
    pub primary_score: Option<f64>,
    pub secondary_score: Option<f64>,
    pub seed: Option<i64>,
}

fn value_string(value: &toml::Value) -> Option<String> {
    value.as_str().map(ToString::to_string)
}

pub fn load_references(path: Option<&Path>) -> Result<Vec<Reference>> {
    let path: PathBuf = path.map(Path::to_path_buf).unwrap_or_else(paths::references_file);
    if !path.is_file() {
        return Err(anyhow!("reference file not found: {}", path.display()));
    }
    let text = paths::read_to_string(&path)?;
    let data: toml::Value = toml::from_str(&text)
        .map_err(|e| anyhow!("{}: invalid TOML: {}", path.display(), e))?;
    let Some(entries) = data.get("reference") else {
        return Ok(Vec::new());
    };
    let Some(entries) = entries.as_array() else {
        return Err(anyhow!("{}: 'reference' must be an array of tables", path.display()));
    };
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for (i, entry) in entries.iter().enumerate() {
        let table = entry
            .as_table()
            .ok_or_else(|| anyhow!("{}: reference #{} must be a table", path.display(), i + 1))?;
        let name = table
            .get("name")
            .and_then(value_string)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| anyhow!("{}: reference #{} needs a name", path.display(), i + 1))?;
        let layout = table
            .get("layout")
            .and_then(value_string)
            .ok_or_else(|| anyhow!("{}: reference #{} needs a layout", path.display(), i + 1))?;
        let letters = layout.chars().filter(|c| c.is_ascii_alphabetic()).count();
        let distinct: HashSet<char> = layout
            .chars()
            .filter(|c| c.is_ascii_alphabetic())
            .collect();
        if layout.chars().count() != 30 || letters != 26 || distinct.len() != 26 {
            return Err(anyhow!(
                "{}: reference {:?} needs 30 keys and 26 distinct letters",
                path.display(),
                name
            ));
        }
        if !seen.insert(name.clone()) {
            return Err(anyhow!("{}: duplicate reference name {:?}", path.display(), name));
        }
        out.push(Reference {
            name,
            layout,
            primary_score: table.get("primary_score").and_then(toml::Value::as_float),
            secondary_score: table.get("secondary_score").and_then(toml::Value::as_float),
            seed: table.get("seed").and_then(toml::Value::as_integer),
        });
    }
    Ok(out)
}

fn toml_string(value: &str) -> String {
    let mut out = String::from("\"");
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn unique_name(name: &str, existing: &HashSet<String>) -> String {
    if !existing.contains(name) {
        return name.to_string();
    }
    let mut i = 2;
    loop {
        let candidate = format!("{}-{}", name, i);
        if !existing.contains(&candidate) {
            return candidate;
        }
        i += 1;
    }
}

/// Append one `[[reference]]` table without rewriting existing content.
pub fn append_reference(
    name: &str,
    layout: &str,
    primary_score: Option<f64>,
    secondary_score: Option<f64>,
    seed: Option<i64>,
    path: Option<&Path>,
) -> Result<Option<String>> {
    let path: PathBuf = path.map(Path::to_path_buf).unwrap_or_else(paths::references_file);
    let refs = load_references(Some(&path))?;
    if refs.iter().any(|r| r.layout == layout) {
        return Ok(None);
    }
    let existing: HashSet<String> = refs.iter().map(|r| r.name.clone()).collect();
    let name = unique_name(name, &existing);
    let mut lines = vec![
        String::new(),
        "[[reference]]".to_string(),
        format!("name = {}", toml_string(&name)),
        format!("layout = {}", toml_string(layout)),
    ];
    if let Some(primary) = primary_score {
        lines.push(format!("primary_score = {:.9}", primary));
    }
    lines.push(format!(
        "secondary_score = {}",
        match secondary_score {
            Some(sec) => format!("{:.9}", sec),
            None => "nan".to_string(),
        }
    ));
    if let Some(seed) = seed {
        lines.push(format!("seed = {}", seed));
    }
    let mut text = std::fs::read_to_string(&path)
        .map_err(|e| anyhow!("{}: {}", path.display(), e))?;
    if !text.is_empty() && !text.ends_with('\n') {
        lines.insert(0, String::new());
        text.push('\n');
    }
    text.push_str(&lines.join("\n"));
    text.push('\n');
    std::fs::write(&path, text).map_err(|e| anyhow!("{}: {}", path.display(), e))?;
    load_references(Some(&path))?;
    Ok(Some(name))
}
