//! Shared command context and corpus resolution.

use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};

use crate::conditions::Conditions;
use crate::config::{self, Config, CorpusEntry};
use crate::corpus::Corpus;
use crate::gates::Gates;
use crate::geometry::Geometry;
use crate::model::Model;
use crate::paths;
use crate::scorer::Tables;

pub struct Context {
    pub cfg: Config,
    pub geom: Geometry,
    pub model: Model,
    pub gates: Gates,
    pub conditions: Conditions,
    pub tables: Tables,
}

impl Context {
    pub fn load(config_path: Option<&Path>) -> Result<Self> {
        let cfg = config::load(config_path)?;
        let geom = Geometry::new();
        let model = config::build_model(&cfg)?;
        let gates = Gates::from_config(&cfg);
        let conditions = Conditions::from_config(&cfg, &geom);
        let tables = Tables::build(&model, &geom, &cfg, &gates, &conditions);
        Ok(Self {
            cfg,
            geom,
            model,
            gates,
            conditions,
            tables,
        })
    }
}

fn expand_user(value: &str) -> PathBuf {
    if let Some(rest) = value.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home).join(rest);
        }
    }
    PathBuf::from(value)
}

pub fn corpus_path(_cfg: &Config, entry: &CorpusEntry) -> PathBuf {
    let value = entry.label();
    let path = expand_user(&value);
    if path.is_file() || path.is_dir() {
        return path;
    }
    let candidate = paths::corpus_root().join(&value).join("parsed");
    if candidate.is_dir() {
        return candidate;
    }
    path
}

pub fn corpus_labels(cfg: &Config, role: &str) -> Vec<String> {
    let entries = if role == "primary" {
        &cfg.primary_corpus
    } else {
        &cfg.secondary_corpus
    };
    entries.iter().map(CorpusEntry::label).collect()
}

pub fn load_role(cfg: &Config, role: &str) -> Result<(Corpus, Vec<String>)> {
    let entries = if role == "primary" {
        &cfg.primary_corpus
    } else {
        &cfg.secondary_corpus
    };
    if entries.is_empty() {
        return Err(anyhow!("config has no {} corpora", role));
    }
    let mut corpora = Vec::new();
    let mut labels = Vec::new();
    for entry in entries {
        let path = corpus_path(cfg, entry);
        labels.push(entry.label());
        corpora.push(Corpus::load(&path)?.scaled(entry.weight));
    }
    Ok((Corpus::merge(&corpora)?, labels))
}

pub fn load_paths(paths_: &[String]) -> Result<Corpus> {
    let paths: Vec<PathBuf> = paths_.iter().map(|p| expand_user(p)).collect();
    crate::corpus::load_many(&paths)
}
