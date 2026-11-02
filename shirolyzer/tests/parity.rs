use std::path::PathBuf;

use shirolyzer::diversity::{self, Row};
use shirolyzer::{conditions, config, corpus, gates, geometry, scorer};

mod common;

const SHIRO: &str = ";ntgfjoehkrpmscuaydilbqvwxz,./";

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
fn trigram_categories_are_exclusive_and_repeats_overlap() {
    let (_cfg, geom, tables, corpus) = setup();
    let pats = shirolyzer::analyze::trigram_patterns(SHIRO, &corpus, &tables, &geom);

    let sft = pats.get("sft").copied().unwrap_or(0.0);
    let sfb_tri = pats.get("sfb_tri").copied().unwrap_or(0.0);
    let sfb_sft = pats.get("sfb_sft").copied().unwrap_or(0.0);
    assert!((sfb_sft - (sft + sfb_tri)).abs() < 1e-9);

    let exclusive = [
        "alternate",
        "inroll",
        "outroll",
        "onehand",
        "redirect",
        "redirect_weak",
        "sft",
        "sfb_tri",
        "other",
    ];
    let total: f64 = exclusive
        .iter()
        .map(|key| pats.get(*key).copied().unwrap_or(0.0))
        .sum();
    assert!((total - 100.0).abs() < 1e-9);

    assert!(
        pats.get("repeat").copied().unwrap_or(0.0) > 0.0,
        "repeat must remain visible as an overlapping diagnostic"
    );
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

#[test]
fn cross_hand_alt_fingers_reach_the_scorer() {
    let mut cfg = config::load(Some(&common::reference_config())).expect("config");
    let h = geometry::key_index("H").unwrap();
    let y = geometry::key_index("Y").unwrap();
    let li = geometry::parse_finger8("LI").unwrap();
    let lm = geometry::parse_finger8("LM").unwrap();
    cfg.alt_fingers = vec![
        config::AltRule {
            keys: vec!['H'],
            fingers: vec![li],
            mode: config::AltMode::Allow,
        },
        config::AltRule {
            keys: vec!['Y'],
            fingers: vec![lm],
            mode: config::AltMode::Allow,
        },
    ];

    let geom = geometry::Geometry::new();
    let model = config::build_model(&cfg).expect("model");
    let gates = gates::Gates::from_config(&cfg);
    let conds = conditions::Conditions::from_config(&cfg, &geom);
    let tables = scorer::Tables::build(&model, &geom, &cfg, &gates, &conds).expect("tables");

    let big = tables.big_f[scorer::big_idx(h, y)] as usize;
    assert_eq!(big / scorer::FINGER_SLOTS, li);
    assert_eq!(big % scorer::FINGER_SLOTS, lm);

    let tri = tables.tri_f[scorer::tri_idx(h, y, h)];
    assert_eq!(tri, [li as u8, lm as u8, li as u8]);
}

#[test]
fn explicit_alt_rules_override_geometry_reachability() {
    let mut cfg = config::load(Some(&common::reference_config())).expect("config");
    let c = geometry::key_index("C").unwrap();
    let h = geometry::key_index("H").unwrap();
    let x = geometry::key_index("X").unwrap();
    let lr = geometry::parse_finger8("LR").unwrap();
    let li = geometry::parse_finger8("LI").unwrap();
    let lp = geometry::parse_finger8("LP").unwrap();
    cfg.alt_fingers = vec![
        config::AltRule {
            keys: vec!['H'],
            fingers: vec![li],
            mode: config::AltMode::Allow,
        },
        config::AltRule {
            keys: vec!['C', 'H', 'X'],
            fingers: vec![lr, li, lp],
            mode: config::AltMode::Allow,
        },
        config::AltRule {
            keys: vec!['H', 'X'],
            fingers: vec![li, lp],
            mode: config::AltMode::Allow,
        },
        config::AltRule {
            keys: vec!['H', 'X', 'X'],
            fingers: vec![li, lp, lp],
            mode: config::AltMode::Allow,
        },
        config::AltRule {
            keys: vec!['X', 'H', 'X'],
            fingers: vec![lp, li, lp],
            mode: config::AltMode::Allow,
        },
    ];

    let geom = geometry::Geometry::new();
    assert!(!geom.is_reachable(c, lr));
    assert!(!geom.is_reachable(x, lp));
    let model = config::build_model(&cfg).expect("model");
    let gates = gates::Gates::from_config(&cfg);
    let conds = conditions::Conditions::from_config(&cfg, &geom);
    let tables = scorer::Tables::build(&model, &geom, &cfg, &gates, &conds).expect("tables");

    let hx = tables.big_f[scorer::big_idx(h, x)] as usize;
    assert_eq!(hx / scorer::FINGER_SLOTS, li);
    assert_eq!(hx % scorer::FINGER_SLOTS, lp);

    let chx = tables.tri_f[scorer::tri_idx(c, h, x)];
    assert_eq!(chx, [lr as u8, li as u8, lp as u8]);
}

#[test]
fn fixed_config_forces_fingermap_fingers() {
    let path = shirolyzer::paths::repo_root().join("shirolyzer/config/config.fixed.yaml");
    let cfg = config::load(Some(&path)).expect("fixed config");
    assert_eq!(cfg.fingering, config::FingeringMode::Fixed);

    let geom = geometry::Geometry::new();
    let model = config::build_model(&cfg).expect("model");
    let gates = gates::Gates::from_config(&cfg);
    let conds = conditions::Conditions::from_config(&cfg, &geom);
    let tables = scorer::Tables::build(&model, &geom, &cfg, &gates, &conds).expect("tables");

    for a in 0..30 {
        for b in 0..30 {
            let encoded = tables.big_f[scorer::big_idx(a, b)] as usize;
            assert_eq!(encoded / scorer::FINGER_SLOTS, cfg.fingermap[a], "{a}->{b}");
            assert_eq!(encoded % scorer::FINGER_SLOTS, cfg.fingermap[b], "{a}->{b}");
        }
    }
    for a in 0..30 {
        for b in 0..30 {
            for c in 0..30 {
                assert_eq!(
                    tables.tri_f[scorer::tri_idx(a, b, c)],
                    [
                        cfg.fingermap[a] as u8,
                        cfg.fingermap[b] as u8,
                        cfg.fingermap[c] as u8
                    ],
                    "{a}->{b}->{c}"
                );
            }
        }
    }
}
