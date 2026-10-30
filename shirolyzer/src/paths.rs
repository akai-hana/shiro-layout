//! Repository-relative path helpers.
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};

fn is_repo_root(path: &Path) -> bool {
    path.join("shirolyzer/config/config.yaml").is_file()
        && path.join("shirolyzer/Cargo.toml").is_file()
}

pub fn repo_root() -> PathBuf {
    if let Ok(dir) = std::env::current_dir() {
        let mut cur = dir.as_path();
        loop {
            if is_repo_root(cur) {
                return cur.to_path_buf();
            }
            match cur.parent() {
                Some(parent) => cur = parent,
                None => break,
            }
        }
    }
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    if is_repo_root(manifest) {
        return manifest.to_path_buf();
    }
    manifest
        .parent()
        .map(Path::to_path_buf)
        .filter(|p| is_repo_root(p))
        .unwrap_or_else(|| manifest.to_path_buf())
}

pub fn config_dir() -> PathBuf {
    repo_root().join("shirolyzer/config")
}

pub fn config_file() -> PathBuf {
    config_dir().join("config.yaml")
}

pub fn model_dir() -> PathBuf {
    config_dir().join("model")
}

pub fn corpus_root() -> PathBuf {
    repo_root().join("corpora")
}

pub fn results_dir() -> PathBuf {
    repo_root().join("shirolyzer/results")
}

pub fn xkb_dir() -> PathBuf {
    repo_root().join("xkb")
}

pub fn references_file() -> PathBuf {
    config_dir().join("references.toml")
}

pub fn rulings_file() -> PathBuf {
    config_dir().join("rulings.json")
}

fn slug(value: &str) -> String {
    let text: String = value
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    let text = text.trim_matches('_');
    if text.is_empty() {
        "layout".to_string()
    } else {
        text.to_string()
    }
}

pub fn result_path(command: &str, name: Option<&str>, suffix: &str) -> PathBuf {
    let stem = match name {
        Some(name) => format!("{}_{}", command, slug(name)),
        None => command.to_string(),
    };
    results_dir().join(format!("{}{}", stem, suffix))
}

pub fn ensure_parent(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    Ok(())
}

pub fn read_to_string(path: &Path) -> Result<String> {
    std::fs::read_to_string(path).map_err(|e| anyhow!("{}: {}", path.display(), e))
}
