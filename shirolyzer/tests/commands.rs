use std::path::PathBuf;
use std::process::Command;

use serde_json::Value;

mod common;

fn root() -> PathBuf {
    shirolyzer::paths::repo_root()
}

fn tmp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("shirolyzer-tests-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir.join(name)
}

fn run(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_shirolyzer"))
        .args(args)
        .current_dir(root())
        .output()
        .expect("run shirolyzer")
}

fn stdout(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn config_path() -> String {
    common::reference_config().to_string_lossy().into_owned()
}

const SHIRO: &str = ";ntgfjoehkrpmscuaydilbqvwxz,./";

#[test]
fn render_matches_reference() {
    let config = config_path();
    let out = run(&["render", SHIRO, "--config", config.as_str()]);
    assert!(out.status.success());
    let expected = ";R nR tM gM fM  jM oM eM hR kR\n rP pR mM sI cI  uI aI yM dR iP\n  lP bR qI vI wI  xI zI ,R .R /P\n";
    assert_eq!(stdout(&out), expected);
}

#[test]
fn xkb_matches_reference() {
    let path = tmp("mine.xkb");
    let out = run(&[
        "xkb",
        "--layout",
        SHIRO,
        "--name",
        "mine",
        "--out",
        path.to_str().unwrap(),
    ]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let generated = std::fs::read_to_string(&path).unwrap();
    let reference = std::fs::read_to_string(root().join("xkb/mine")).unwrap();
    assert_eq!(generated, reference);
}

#[test]
fn analyze_score_matches_baseline() {
    let config = config_path();
    let path = tmp("analyze.json");
    let out = run(&[
        "analyze",
        SHIRO,
        "--config",
        config.as_str(),
        "--out-json",
        path.to_str().unwrap(),
    ]);
    assert!(out.status.success());
    let value: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    let score = value["primary_score"].as_f64().unwrap();
    assert!((score - 3.628_766_121_938_924).abs() < 1e-12);
    for entry in value["guide"].as_object().unwrap().values() {
        for contexts in entry["alt_contexts"].as_object().unwrap().values() {
            for item in contexts.as_array().unwrap() {
                assert!(
                    item[1].is_i64(),
                    "trigram count should serialize as an integer"
                );
            }
        }
    }
    let pats = value["trigram_patterns"].as_object().unwrap();
    for key in [
        "redirect_tier_1",
        "redirect_tier_5",
        "redirect_1_2_1",
        "redirect_unclassified",
        "total_redirects",
    ] {
        assert!(pats.contains_key(key), "missing {key}");
    }
    let tier_total: f64 = (1..=5)
        .map(|tier| pats[&format!("redirect_tier_{tier}")].as_f64().unwrap())
        .sum();
    let unclassified = pats["redirect_unclassified"].as_f64().unwrap();
    let total = pats["total_redirects"].as_f64().unwrap();
    assert!((tier_total + unclassified - total).abs() < 1e-9);

    // Bigram roll shares by directed finger pair always sum to the in/out rolls
    // that are neither leaps nor wide jumps.
    let cats = value["categories"].as_object().unwrap();
    let pair_total: f64 = ["1_2", "2_1", "2_3", "3_2", "3_4", "4_3"]
        .iter()
        .map(|pair| cats[&format!("roll_{pair}")].as_f64().unwrap())
        .sum();
    let rolls = cats["roll_in"].as_f64().unwrap() + cats["roll_out"].as_f64().unwrap();
    assert!(pair_total > 0.0 && pair_total <= rolls + 1e-9);
    assert!(
        !cats.contains_key("roll_tier_1"),
        "reference config has no roll tiers"
    );

    let provenance = &value["corpus_provenance"];
    for role in ["primary", "secondary"] {
        let entries = provenance[role].as_array().unwrap();
        assert_eq!(entries.len(), 1, "{role} provenance");
        let entry = &entries[0];
        assert_eq!(entry["kind"], "parsed");
        assert_eq!(entry["parser"], "parsed-json-v1");
        assert_eq!(entry["normalization"], "none (pre-parsed counts)");
        assert_eq!(entry["files"].as_array().unwrap().len(), 3);
        assert_eq!(entry["combined_sha1"].as_str().unwrap().len(), 40);
        assert!(entry["totals"]["trigram_entries"].as_u64().unwrap() > 0);
    }
}

#[test]
fn compare_accepts_external_ngram_json() {
    let config = config_path();
    let path = tmp("external-ngrams.txt");
    std::fs::write(
        &path,
        r#"{"letters": {"a": 3}, "bigrams": {"ab": 2}, "trigrams": {"abc": 1}}"#,
    )
    .unwrap();
    let out = run(&[
        "compare",
        "--config",
        config.as_str(),
        "--corpus-json",
        path.to_str().unwrap(),
    ]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = stdout(&out);
    assert!(text.contains("shiro"), "{text}");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("external corpus"), "{err}");
    assert!(err.contains("external-json"), "{err}");
}

#[test]
fn worst_lists_highest_cost_ngrams() {
    let config = config_path();
    let layout = "ztiuqvgrnhekyaocslpd;j,./wxbfm";
    let out = run(&["worst", layout, "--config", config.as_str(), "--limit", "1"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = stdout(&out);
    assert!(text.contains("showing 1 of 676 ngrams"));
    assert!(text.contains("xv"));
    assert!(text.contains("M Y"));
    assert!(text.contains("RM-RI"));
    assert!(text.contains("showing 1 of 15437 ngrams"));
    assert!(text.contains("rnp"));
    assert!(text.contains("I O L"));
    assert!(text.contains("onehand"));
}

#[test]
fn worst_reports_redirect_tiers() {
    let config = config_path();
    let layout = "ztiuqvgrnhekyaocslpd;j,./wxbfm";
    let out = run(&[
        "worst",
        layout,
        "--config",
        config.as_str(),
        "--limit",
        "20",
    ]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = stdout(&out);
    assert!(text.contains("redirect_tier_"), "{text}");
    assert!(text.contains("redirect_unclassified:"), "{text}");
}

#[test]
fn compare_reports_baseline_scores() {
    let config = config_path();
    let out = run(&["compare", "--config", config.as_str()]);
    assert!(out.status.success());
    let text = stdout(&out);
    assert!(text.contains("shiro"));
    assert!(text.contains("3.628766"));
}

#[test]
fn audit_and_check_baseline_pass() {
    let config = config_path();
    let out_json = tmp("audit.json");
    let out_txt = tmp("audit.txt");
    let out = run(&[
        "audit",
        "A:all",
        "--config",
        config.as_str(),
        "--out",
        out_txt.to_str().unwrap(),
        "--out-json",
        out_json.to_str().unwrap(),
    ]);
    assert!(out.status.success());
    assert!(stdout(&out).contains("rulings: 166/166"));
    let value: Value = serde_json::from_str(&std::fs::read_to_string(&out_json).unwrap()).unwrap();
    assert_eq!(value["config"], "A:all");
    for corpus in value["corpora"].as_object().unwrap().values() {
        for key in ["letters", "bigrams", "trigrams"] {
            assert!(
                corpus[key].is_i64(),
                "corpus {key} should serialize as an integer"
            );
        }
    }

    let default_json = tmp("audit-default.json");
    let default_txt = tmp("audit-default.txt");
    let out = run(&[
        "audit",
        "--config",
        config.as_str(),
        "--out",
        default_txt.to_str().unwrap(),
        "--out-json",
        default_json.to_str().unwrap(),
    ]);
    assert!(out.status.success());
    let value: Value =
        serde_json::from_str(&std::fs::read_to_string(&default_json).unwrap()).unwrap();
    assert!(value["config"].is_null());

    let out = run(&["check-baseline", "--config", config.as_str()]);
    assert!(out.status.success());
    assert!(stdout(&out).contains("baseline OK: audit 166/166, 9 references"));
}

#[test]
#[ignore = "slow in debug builds; run with cargo test --release -- --ignored"]
fn compute_smoke() {
    let refs = tmp("refs.toml");
    std::fs::write(
        &refs,
        format!("[[reference]]\nname = \"shiro\"\nlayout = \"{SHIRO}\"\n"),
    )
    .unwrap();
    let config = config_path();
    let out_json = tmp("compute.json");
    let out_txt = tmp("compute.txt");
    let out = run(&[
        "compute",
        "--config",
        config.as_str(),
        "--references",
        refs.to_str().unwrap(),
        "--seed-reference",
        "shiro",
        "--polish-rounds",
        "0",
        "--runs",
        "0",
        "--iters",
        "10",
        "--ils",
        "1",
        "--seed",
        "123",
        "--out",
        out_txt.to_str().unwrap(),
        "--out-json",
        out_json.to_str().unwrap(),
    ]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let value: Value = serde_json::from_str(&std::fs::read_to_string(&out_json).unwrap()).unwrap();
    assert_eq!(value["completed_runs"], 1);
    assert_eq!(value["valid_layouts"].as_array().unwrap().len(), 1);
    let primary = value["best_primary"]["primary"].as_f64().unwrap();
    assert!((primary - 3.576_952_749_945_849_8).abs() < 1e-6);
}

#[test]
fn analyze_reports_configured_roll_tiers() {
    let path = tmp("roll-tiers-config.yaml");
    let mut text = std::fs::read_to_string(common::reference_config()).unwrap();
    text.push_str("\nroll_tiers:\n  - [\"1-2\"]\n  - [\"2-1\", \"3-2\"]\n");
    std::fs::write(&path, text).unwrap();
    let json = tmp("roll-tiers.json");
    let out = run(&[
        "analyze",
        SHIRO,
        "--config",
        path.to_str().unwrap(),
        "--out-json",
        json.to_str().unwrap(),
    ]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = stdout(&out);
    assert!(text.contains("== roll tiers (configured"), "{text}");
    let value: Value = serde_json::from_str(&std::fs::read_to_string(&json).unwrap()).unwrap();
    let cats = value["categories"].as_object().unwrap();
    let pair = |key: &str| cats[key].as_f64().unwrap();
    assert!((cats["roll_tier_1"].as_f64().unwrap() - pair("roll_1_2")).abs() < 1e-9);
    assert!(
        (cats["roll_tier_2"].as_f64().unwrap() - pair("roll_2_1") - pair("roll_3_2")).abs() < 1e-9
    );
}
