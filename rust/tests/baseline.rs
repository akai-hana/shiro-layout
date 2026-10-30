use std::path::PathBuf;

use serde::Deserialize;
use shirolyzer::{conditions, config, corpus, evaluate, gates, geometry, scorer};

#[derive(Deserialize)]
struct Reference {
    name: String,
    layout: String,
}

#[derive(Deserialize)]
struct References {
    reference: Vec<Reference>,
}

fn setup() -> (config::Config, geometry::Geometry, scorer::Tables, corpus::Corpus) {
    let cfg = config::load(None).expect("config");
    let geom = geometry::Geometry::new();
    let model = config::build_model(&cfg).expect("model");
    let gates = gates::Gates::from_config(&cfg);
    let conds = conditions::Conditions::from_config(&cfg, &geom);
    let tables = scorer::Tables::build(&model, &geom, &cfg, &gates, &conds);
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
    let (cfg, _geom, tables, corpus) = setup();
    let expected: &[(&str, f64)] = &[
        ("qwerty", 4.2669630602387265),
        ("colemak", 3.820607408434575),
        ("colemak-dh", 3.812332615778344),
        ("canary", 3.66859916313943),
        ("sturdy", 3.63996349703581),
        ("gallium", 3.5885065846421957),
        ("semimak", 3.622449779272503),
        ("shiro", 3.3940533330472595),
        ("t5-diversity-2", 3.3956351604837973),
    ];
    let refs_text = std::fs::read_to_string(shirolyzer::paths::references_file()).unwrap();
    let refs: References = toml::from_str(&refs_text).unwrap();
    let primary = evaluate::score_layout(&cfg.current_layout, &corpus, &tables);
    println!("config current: {primary:.12} expected 3.394053333047");
    let mut worst = 0.0f64;
    for reference in &refs.reference {
        let Some((_, exp)) = expected.iter().find(|(n, _)| *n == reference.name) else {
            continue;
        };
        let score = evaluate::score_layout(&reference.layout, &corpus, &tables);
        let diff = (score - exp).abs();
        worst = worst.max(diff);
        println!("{:14} {score:.12} expected {exp:.12} diff {:.3e}", reference.name, diff);
    }
    println!("worst diff {worst:.3e}");
}
