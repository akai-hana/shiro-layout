use std::path::PathBuf;

use serde::Deserialize;
use shirolyzer::{conditions, config, corpus, evaluate, gates, geometry, scorer};

mod common;

const SHIRO: &str = "gwhtsmuenqlrdv;oajpicxkfz,.by/";

#[derive(Deserialize)]
struct Reference {
    name: String,
    layout: String,
}

#[derive(Deserialize)]
struct References {
    reference: Vec<Reference>,
}

fn setup() -> (
    config::Config,
    geometry::Geometry,
    scorer::Tables,
    corpus::Corpus,
) {
    let cfg = config::load(Some(&common::reference_config())).expect("config");
    let geom = geometry::Geometry::new();
    let model = config::build_model(&cfg).expect("model");
    let gates = gates::Gates::from_config(&cfg);
    let conds = conditions::Conditions::from_config(&cfg, &geom);
    let tables = scorer::Tables::build(&model, &geom, &cfg, &gates, &conds).expect("tables");
    let paths: Vec<PathBuf> = cfg
        .primary_corpus
        .iter()
        .map(|e| cfg.corpus_path(e))
        .collect();
    let corpus = corpus::load_many(&paths).expect("corpus");
    (cfg, geom, tables, corpus)
}

#[test]
fn reference_trigram_scores() {
    let (_cfg, _geom, tables, corpus) = setup();
    let expected: &[(&str, f64)] = &[
        ("qwerty", 4.945100363667683),
        ("colemak", 4.161659802668337),
        ("colemak-dh", 4.1601932849635785),
        ("canary", 4.001729813779553),
        ("sturdy", 3.9152460321534845),
        ("gallium", 3.871559718134498),
        ("semimak", 3.947694742590448),
        ("shiro", 3.907_860_887_460_963),
        ("t5-diversity-2", 3.664580151719405),
    ];
    let refs_text = std::fs::read_to_string(shirolyzer::paths::references_file()).unwrap();
    let refs: References = toml::from_str(&refs_text).unwrap();
    let primary = evaluate::score_layout(SHIRO, &corpus, &tables);
    assert!(
        (primary - 3.907_860_887_460_963).abs() < 1e-9,
        "configured current layout score {primary}"
    );
    let mut worst = 0.0f64;
    let mut checked = 0usize;
    for reference in &refs.reference {
        let Some((_, exp)) = expected.iter().find(|(n, _)| *n == reference.name) else {
            continue;
        };
        let score = evaluate::score_layout(&reference.layout, &corpus, &tables);
        let diff = (score - exp).abs();
        worst = worst.max(diff);
        checked += 1;
        println!(
            "{:14} {score:.12} expected {exp:.12} diff {:.3e}",
            reference.name, diff
        );
    }
    assert_eq!(checked, expected.len(), "reference fixture names changed");
    assert!(worst < 1e-9, "worst reference score diff {worst:.3e}");
}
