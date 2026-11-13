//! `compare`: score reference layouts against a corpus.

use std::path::Path;

use anyhow::Result;

use crate::context;
use crate::evaluate::score_layout;
use crate::provenance;
use crate::references;

pub fn run(
    config: Option<&Path>,
    primary_corpus: Option<&str>,
    corpus_json: Option<&Path>,
    refs_path: Option<&Path>,
) -> Result<()> {
    let ctx = context::Context::load(config)?;
    let corpus = if let Some(path) = corpus_json {
        let source = provenance::describe_external("external", 1.0, path)?;
        eprintln!(
            "external corpus {} ({}) sha1={}",
            source.path.display(),
            source.kind,
            source.combined_sha1
        );
        crate::corpus::Corpus::load_external(path)?
    } else {
        match primary_corpus {
            Some(path) => crate::corpus::Corpus::load(Path::new(path))?,
            None => context::load_role(&ctx.cfg, "primary")?.0,
        }
    };
    eprintln!("loaded model {}", ctx.cfg.parameters_path.display());
    if ctx.cfg.sfb_impossible {
        eprintln!("warning: compare is using a config with general.sfb_impossible=true;");
        eprintln!(
            "         any reference layout with a same-finger bigram receives a fixed 1e9 penalty,"
        );
        eprintln!("         so these rows are not comparable across layouts.");
        eprintln!("         Re-run with --config config/config.fixed.yaml or another permissive");
        eprintln!(
            "         config that sets general.sfb_impossible: false for cross-layout scores."
        );
    }
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
