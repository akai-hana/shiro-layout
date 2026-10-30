//! `compare`: score reference layouts against a corpus.

use std::path::Path;

use anyhow::Result;

use crate::context;
use crate::evaluate::score_layout;
use crate::references;

pub fn run(
    config: Option<&Path>,
    primary_corpus: Option<&str>,
    refs_path: Option<&Path>,
) -> Result<()> {
    let ctx = context::Context::load(config)?;
    let corpus = match primary_corpus {
        Some(path) => crate::corpus::Corpus::load(Path::new(path))?,
        None => context::load_role(&ctx.cfg, "primary")?.0,
    };
    eprintln!(
        "loaded model {}",
        ctx.cfg.parameters_path.display()
    );
    let refs = references::load_references(refs_path)?;
    let mut rows: Vec<(f64, String, String)> = refs
        .iter()
        .map(|entry| {
            (
                score_layout(&entry.layout, &corpus, &ctx.tables),
                entry.name.clone(),
                entry.layout.clone(),
            )
        })
        .collect();
    rows.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    println!("{:<14} {:>12}  keys", "layout", "score");
    for (cost, name, layout) in rows {
        println!("{:<14} {:>12.6}  {}", name, cost, layout);
    }
    Ok(())
}
