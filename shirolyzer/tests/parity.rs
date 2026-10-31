use std::path::PathBuf;

use shirolyzer::diversity::{self, Row};
use shirolyzer::{conditions, config, corpus, gates, geometry, scorer};

const SHIRO: &str = ";ntgfjoehkrpmscuaydilbqvwxz,./";

fn setup() -> (
    config::Config,
    geometry::Geometry,
    scorer::Tables,
    corpus::Corpus,
) {
    let cfg = config::load(None).expect("config");
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
fn tables_have_expected_shape_and_values() {
    let (_cfg, _geom, tables, _corpus) = setup();
    assert_eq!(tables.big.len(), 31 * 31);
    assert_eq!(tables.tri.len(), 31 * 31 * 31);
    let tri = |a: usize, b: usize, c: usize| tables.tri[a * 961 + b * 31 + c];
    let cases = [
        ((0, 0, 0), 4.800000000000001),
        ((0, 1, 2), 13.583620387057007),
        ((5, 10, 20), 7.101148117967329),
        ((10, 10, 10), 4.2),
        ((30, 30, 30), 1.56),
        ((29, 28, 27), 4.646922177562787),
    ];
    for ((a, b, c), expected) in cases {
        assert!((tri(a, b, c) - expected).abs() < 1e-12, "{a},{b},{c}");
    }
    let big = |a: usize, b: usize| tables.big[a * 31 + b];
    for ((a, b), expected) in [
        ((0, 0), 3.0),
        ((0, 1), 2.583620377973183),
        ((5, 10), 2.283062007216651),
        ((30, 30), 0.0),
        ((29, 28), 3.576544872166762),
    ] {
        assert!((big(a, b) - expected).abs() < 1e-12, "{a},{b}");
    }
}

#[test]
fn diversity_keeps_only_comparable_distinct_layouts() {
    let reference = Row::new("ref", 3.5202, Some(3.7048), SHIRO);
    let near = Row::new(
        "near",
        3.5210,
        Some(3.7050),
        ";ntgfjoehkrpmscuaydilbqvwxz,..",
    );
    let far = Row::new(
        "far",
        3.5215,
        Some(3.7060),
        "qwertyuiopasdfghjkl;zxcvbnm,./",
    );
    let outside = Row::new(
        "outside",
        3.5230,
        Some(3.7048),
        "abcdefghijklmnopqrstuvwxyz;/,.",
    );
    let rows = vec![
        reference.clone(),
        near.clone(),
        far.clone(),
        outside.clone(),
    ];
    let (selected, truncated) =
        diversity::select_valid_layouts(&rows, &reference, 0.002, 0.005, 4, 8);
    assert!(!truncated);
    let layouts: Vec<&str> = selected.iter().map(|r| r.layout.as_str()).collect();
    assert_eq!(layouts.first().copied(), Some(SHIRO));
    assert!(layouts.contains(&far.layout.as_str()));
    assert!(!layouts.contains(&near.layout.as_str()));
    assert!(!layouts.contains(&outside.layout.as_str()));
}
