//! `check-baseline`: verify reference scores and audit rulings.

use std::path::Path;

use anyhow::{anyhow, Result};
use serde_json::{json, Value};

use crate::audit;
use crate::context::{self, Context};
use crate::corpus::Corpus;
use crate::evaluate::remap_for_layout;
use crate::paths;
use crate::references;
use crate::scorer::{big_idx, tri_idx, Tables};

pub struct CheckArgs<'a> {
    pub baseline: Option<&'a Path>,
    pub corpus: Option<&'a Path>,
    pub tolerance: f64,
    pub write: bool,
}

pub fn reference_costs(layout: &str, corpus: &Corpus, tables: &Tables) -> Result<Value> {
    if layout.chars().count() != 30 {
        return Err(anyhow!("{:?} is not a 30-character layout", layout));
    }
    let remap = remap_for_layout(layout);
    let mut btotal = 0.0;
    let mut bweighted = 0.0;
    for (ids, &count) in corpus.bi_ids.iter().zip(corpus.bi_counts.iter()) {
        let a = remap[ids[0]];
        let b = remap[ids[1]];
        if a < 0 || b < 0 {
            continue;
        }
        btotal += count;
        bweighted += tables.big[big_idx(a as usize, b as usize)] * count;
    }
    let mut ttotal = 0.0;
    let mut tweighted = 0.0;
    for (ids, &count) in corpus.tri_ids.iter().zip(corpus.tri_counts.iter()) {
        let a = remap[ids[0]];
        let b = remap[ids[1]];
        let c = remap[ids[2]];
        if a < 0 || b < 0 || c < 0 {
            continue;
        }
        ttotal += count;
        tweighted += tables.tri[tri_idx(a as usize, b as usize, c as usize)] * count;
    }
    Ok(json!({
        "bigram": if btotal == 0.0 { 0.0 } else { bweighted / btotal },
        "trigram": if ttotal == 0.0 { 0.0 } else { tweighted / ttotal },
    }))
}

fn collect(ctx: &Context, corpus_path: &Path) -> Result<Value> {
    let corpus = Corpus::load(corpus_path)?;
    let audit_outcome = audit::collect(&ctx, "A:all", &[], &[], None, true)?;
    let model = ctx
        .cfg
        .parameters_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("pattern_parameters_A_all.json")
        .to_string();
    let mut refs = serde_json::Map::new();
    for entry in references::load_references(None)? {
        refs.insert(
            entry.name.clone(),
            reference_costs(&entry.layout, &corpus, &ctx.tables)?,
        );
    }
    Ok(json!({
        "model": model,
        "audit": {
            "config": "A:all",
            "passed": audit_outcome.rulings_passed,
            "total": audit_outcome.rulings_total,
        },
        "references": refs,
    }))
}

fn compare(expected: &Value, actual: &Value, tolerance: f64) -> Vec<String> {
    let mut errors = Vec::new();
    for field in ["passed", "total"] {
        let want = &expected["audit"][field];
        let got = &actual["audit"][field];
        if want != got {
            errors.push(format!("audit.{}: {} != {}", field, want, got));
        }
    }
    if expected["model"] != actual["model"] {
        errors.push(format!(
            "model: {} != {}",
            expected["model"], actual["model"]
        ));
    }
    let expected_refs = expected["references"].as_object().cloned().unwrap_or_default();
    let actual_refs = actual["references"].as_object().cloned().unwrap_or_default();
    for name in expected_refs.keys() {
        if !actual_refs.contains_key(name) {
            errors.push(format!("missing references: [\"{}\"]", name));
        }
    }
    let mut names: Vec<&String> = expected_refs
        .keys()
        .filter(|n| actual_refs.contains_key(*n))
        .collect();
    names.sort();
    for name in names {
        for field in ["bigram", "trigram"] {
            let want = expected_refs[name][field].as_f64().unwrap_or(0.0);
            let got = actual_refs[name][field].as_f64().unwrap_or(0.0);
            if (want - got).abs() > tolerance {
                errors.push(format!("{}.{}: {:.12} != {:.12}", name, field, want, got));
            }
        }
    }
    errors
}

pub fn run(args: CheckArgs) -> Result<()> {
    let baseline = args
        .baseline
        .map(Path::to_path_buf)
        .unwrap_or_else(|| paths::config_dir().join("score_baseline.json"));
    let corpus_path = args
        .corpus
        .map(Path::to_path_buf)
        .unwrap_or_else(|| paths::corpus_root().join("shai").join("parsed"));
    let ctx = context::Context::load(None)?;
    let actual = collect(&ctx, &corpus_path)?;
    if args.write {
        if let Some(parent) = baseline.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&baseline, serde_json::to_string_pretty(&actual)? + "\n")?;
        println!("wrote {}", baseline.display());
        return Ok(());
    }
    if !baseline.is_file() {
        return Err(anyhow!("baseline not found: {}", baseline.display()));
    }
    let expected: Value = serde_json::from_str(&paths::read_to_string(&baseline)?)?;
    let errors = compare(&expected, &actual, args.tolerance);
    if !errors.is_empty() {
        for error in errors {
            println!("FAIL {}", error);
        }
        return Err(anyhow!("baseline check failed"));
    }
    println!(
        "baseline OK: audit {}/{}, {} references",
        actual["audit"]["passed"],
        actual["audit"]["total"],
        actual["references"].as_object().map(|m| m.len()).unwrap_or(0)
    );
    Ok(())
}
