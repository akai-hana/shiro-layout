//! Repository-relative path helpers.
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

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

pub fn geometry_dir() -> PathBuf {
    repo_root().join("shirolyzer/geometry")
}

pub fn geometry_file(name: &str) -> PathBuf {
    geometry_dir().join(format!("{}.yaml", name))
}

pub fn corpus_root() -> PathBuf {
    repo_root().join("corpora")
}

pub fn results_dir() -> PathBuf {
    repo_root().join("shirolyzer/results")
}

pub fn latest_layout_file() -> PathBuf {
    results_dir().join("latest")
}

pub fn read_latest_layout() -> Result<String> {
    let path = latest_layout_file();
    let text = read_to_string(&path).map_err(|_| {
        anyhow!(
            "{}: no generated layout found; run `shirolyzer compute` first or pass a layout",
            path.display()
        )
    })?;
    Ok(text.trim().to_string())
}

pub fn write_latest_layout(layout: &str, timestamp: &str) -> Result<PathBuf> {
    let path = results_dir().join(format!("layout_{}.txt", timestamp));
    ensure_parent(&path)?;
    std::fs::write(&path, format!("{}\n", layout))?;
    let link = latest_layout_file();
    if std::fs::symlink_metadata(&link).is_ok() {
        std::fs::remove_file(&link)?;
    }
    #[cfg(unix)]
    std::os::unix::fs::symlink(path.file_name().unwrap(), &link)?;
    #[cfg(not(unix))]
    std::fs::copy(&path, &link)?;
    Ok(path)
}

/// UTC timestamp in `YYYYMMDD-HHMMSS` form, without pulling in a date crate.
pub fn timestamp() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs() as i64;
    let days = secs.div_euclid(86_400);
    let sod = secs.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    format!(
        "{:04}{:02}{:02}-{:02}{:02}{:02}",
        year,
        month,
        day,
        sod / 3600,
        (sod % 3600) / 60,
        sod % 60
    )
}

fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    ((if m <= 2 { y + 1 } else { y }), m as u32, d as u32)
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
