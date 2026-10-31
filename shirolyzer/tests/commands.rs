use std::path::PathBuf;
use std::process::Command;

use serde_json::Value;

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

const SHIRO: &str = ";ntgfjoehkrpmscuaydilbqvwxz,./";

#[test]
fn render_matches_reference() {
    let out = run(&["render", SHIRO]);
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
    let path = tmp("analyze.json");
    let out = run(&["analyze", SHIRO, "--out-json", path.to_str().unwrap()]);
    assert!(out.status.success());
    let value: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    let score = value["primary_score"].as_f64().unwrap();
    assert!((score - 3.616309837353795).abs() < 1e-12);
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
}

#[test]
fn compare_reports_baseline_scores() {
    let out = run(&["compare"]);
    assert!(out.status.success());
    let text = stdout(&out);
    assert!(text.contains("shiro"));
    assert!(text.contains("3.616310"));
}

#[test]
fn audit_and_check_baseline_pass() {
    let out_json = tmp("audit.json");
    let out_txt = tmp("audit.txt");
    let out = run(&[
        "audit",
        "A:all",
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
        "--out",
        default_txt.to_str().unwrap(),
        "--out-json",
        default_json.to_str().unwrap(),
    ]);
    assert!(out.status.success());
    let value: Value =
        serde_json::from_str(&std::fs::read_to_string(&default_json).unwrap()).unwrap();
    assert!(value["config"].is_null());

    let out = run(&["check-baseline"]);
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
    let out_json = tmp("compute.json");
    let out_txt = tmp("compute.txt");
    let out = run(&[
        "compute",
        "--references",
        refs.to_str().unwrap(),
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
    assert_eq!(value["valid_layouts"].as_array().unwrap().len(), 2);
    let primary = value["best_primary"]["primary"].as_f64().unwrap();
    assert!((primary - 3.5528408956196365).abs() < 1e-6);
}
