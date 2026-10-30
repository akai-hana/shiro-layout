//! `audit`: questionnaire and ruling audit for the pattern model.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use anyhow::Result;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::context::Context;
use crate::geometry::{finger_index, key_index, FINGERS, KEYS, NF};
use crate::model::{pair_feature, pt_idx};
use crate::paths;
use crate::scorer::PRESS;

#[derive(Debug, Deserialize, Clone)]
pub struct Ruling {
    pub id: String,
    pub kind: String,
    pub data: Value,
    #[serde(default)]
    pub src: String,
    #[serde(default)]
    pub family: String,
    #[serde(default)]
    pub note: String,
    #[serde(default)]
    pub soft: bool,
}

pub struct AuditOutcome {
    pub rulings_passed: usize,
    pub rulings_total: usize,
    pub probes_passed: usize,
    pub probes_total: usize,
    pub corpora: serde_json::Value,
    pub anchors: Vec<String>,
    pub lines: Vec<String>,
}

fn finger(name: &str) -> usize {
    finger_index(name.chars().next().unwrap()).unwrap()
}

fn key(name: &str) -> usize {
    key_index(name).unwrap()
}

fn t2(ctx: &Context, a: usize, fa: usize, b: usize, fb: usize) -> f64 {
    ctx.tables.model.t2[pt_idx(a, fa, b, fb)]
}

fn pt(ctx: &Context, a: usize, fa: usize, b: usize, fb: usize) -> f64 {
    ctx.tables.model.pt[pt_idx(a, fa, b, fb)]
}

fn fp(fs: &[usize]) -> String {
    fs.iter().map(|&f| FINGERS[f]).collect()
}

pub fn load_rulings(path: Option<&Path>) -> Result<Vec<Ruling>> {
    let path = path
        .map(Path::to_path_buf)
        .unwrap_or_else(paths::rulings_file);
    let text = paths::read_to_string(&path)?;
    let mut rulings: Vec<Ruling> = serde_json::from_str(&text)?;
    // The shipped JSON contains 168 entries; the two unresolved J/Y twins are
    // disabled by default (fitlib.UNRESOLVED_FAMILIES).
    rulings.retain(|r| r.family != "R13_JY" || std::env::var("SHIRO_KEEP_UNRESOLVED").is_ok());
    Ok(rulings)
}

fn block(ctx: &Context, keys: &[usize]) -> Vec<f64> {
    if keys.len() == 2 {
        let (a, b) = (keys[0], keys[1]);
        let mut out = Vec::with_capacity(NF * NF);
        for fa in 0..NF {
            for fb in 0..NF {
                if a == b && fa != fb {
                    out.push(f64::INFINITY);
                } else {
                    out.push(t2(ctx, a, fa, b, fb));
                }
            }
        }
        out
    } else {
        ctx.model.tri_many(
            &ctx.geom,
            &ctx.tables.model,
            &[keys[0]],
            &[keys[1]],
            &[keys[2]],
        )
    }
}

fn penalty(ctx: &Context, names: &[&str]) -> Vec<f64> {
    let chars: Vec<char> = names.iter().map(|s| s.chars().next().unwrap()).collect();
    ctx.gates.penalty_mask(&chars, &ctx.conditions)
}

/// Evaluate one ruling: (loss, ok, detail).
pub fn eval_ruling(ctx: &Context, r: &Ruling) -> (f64, bool, String) {
    let w = if r.soft { 0.5 } else { 1.0 };
    match r.kind.as_str() {
        "top1" => {
            let typ = r.data[0].as_str().unwrap_or("");
            let keys: Vec<&str> = r.data[1]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_str().unwrap())
                .collect();
            let want = r.data[2].as_str().unwrap_or("");
            if typ == "bi" {
                let a = key(keys[0]);
                let b = key(keys[1]);
                let pm = penalty(ctx, &keys);
                let mut cand: Vec<(f64, String)> = Vec::new();
                for fa in 0..NF {
                    for fb in 0..NF {
                        let c = t2(ctx, a, fa, b, fb);
                        if c.is_finite() {
                            cand.push((c + pm[fa * NF + fb], fp(&[fa, fb])));
                        }
                    }
                }
                cand.sort_by(|x, y| {
                    x.0.partial_cmp(&y.0)
                        .unwrap_or(std::cmp::Ordering::Equal)
                        .then_with(|| x.1.cmp(&y.1))
                });
                let tgt = cand.iter().find(|(_, f)| f == want).map(|(c, _)| *c);
                let Some(ct) = tgt else {
                    return (
                        5.0 * w,
                        false,
                        format!("{} want {}: not reachable", keys.join("->"), want),
                    );
                };
                let others: Vec<f64> = cand
                    .iter()
                    .filter(|(_, f)| f != want)
                    .map(|(c, _)| *c)
                    .take(6)
                    .collect();
                let loss: f64 = others.iter().map(|c| (0.05 + ct - c).max(0.0)).sum();
                let ok = cand[0].1 == want;
                (
                    w * loss,
                    ok,
                    format!(
                        "{} want {} got {} ({:.2}; target {:.2})",
                        keys.join("->"),
                        want,
                        cand[0].1,
                        cand[0].0,
                        ct
                    ),
                )
            } else {
                let ids: Vec<usize> = keys.iter().map(|k| key(k)).collect();
                let mut vals = block(ctx, &ids);
                let pm = penalty(ctx, &keys);
                for (i, v) in vals.iter_mut().enumerate() {
                    *v += pm[i];
                }
                let mut cand: Vec<(f64, String)> = Vec::new();
                for f1 in 0..NF {
                    for f2 in 0..NF {
                        for f3 in 0..NF {
                            let c = vals[(f1 * NF + f2) * NF + f3];
                            if c.is_finite() {
                                cand.push((c, fp(&[f1, f2, f3])));
                            }
                        }
                    }
                }
                cand.sort_by(|x, y| {
                    x.0.partial_cmp(&y.0)
                        .unwrap_or(std::cmp::Ordering::Equal)
                        .then_with(|| x.1.cmp(&y.1))
                });
                let tgt = cand.iter().find(|(_, f)| f == want).map(|(c, _)| *c);
                let Some(ct) = tgt else {
                    return (
                        5.0 * w,
                        false,
                        format!("{} want {}: not reachable", keys.join("->"), want),
                    );
                };
                let others: Vec<f64> = cand
                    .iter()
                    .filter(|(_, f)| f != want)
                    .map(|(c, _)| *c)
                    .take(6)
                    .collect();
                let loss: f64 = others.iter().map(|c| (0.05 + ct - c).max(0.0)).sum();
                let ok = cand[0].1 == want;
                (
                    w * loss,
                    ok,
                    format!(
                        "{} want {} got {} ({:.2}; target {:.2})",
                        keys.join("->"),
                        want,
                        cand[0].1,
                        cand[0].0,
                        ct
                    ),
                )
            }
        }
        "key1" => {
            let k = key(r.data[0].as_str().unwrap());
            let fing = r.data[1].as_str().unwrap();
            let e = &ctx.tables.model.e[k * NF..(k + 1) * NF];
            let ct = e[finger(fing)];
            let mut others: Vec<(f64, String)> = Vec::new();
            for i in 0..NF {
                let f = FINGERS[i].to_string();
                if f != fing && e[i].is_finite() {
                    others.push((e[i], f));
                }
            }
            let loss: f64 = others.iter().map(|(c, _)| (0.03 + ct - c).max(0.0)).sum();
            let mut all = others.clone();
            all.push((ct, fing.to_string()));
            all.sort_by(|x, y| {
                x.0.partial_cmp(&y.0)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| x.1.cmp(&y.1))
            });
            (
                loss,
                all[0].1 == fing,
                format!(
                    "{}: want {} got {} ({:.2}; default {:.2})",
                    r.data[0].as_str().unwrap(),
                    fing,
                    all[0].1,
                    all[0].0,
                    ct
                ),
            )
        }
        "order" => {
            let items = r.data.as_array().unwrap();
            let costs: Vec<f64> = items
                .iter()
                .map(|it| {
                    let a = it.as_array().unwrap();
                    t2(
                        ctx,
                        key(a[0].as_str().unwrap()),
                        finger(a[1].as_str().unwrap()),
                        key(a[2].as_str().unwrap()),
                        finger(a[3].as_str().unwrap()),
                    )
                })
                .collect();
            let m = if r.family == "RPDIR" { 0.15 } else { 0.02 };
            let loss: f64 = costs.windows(2).map(|w| (m + w[0] - w[1]).max(0.0)).sum();
            let ok = costs.windows(2).all(|w| w[0] + 1e-9 < w[1]);
            (
                w * loss,
                ok,
                format!(
                    "order {}",
                    costs
                        .iter()
                        .map(|c| format!("{:.2}", c))
                        .collect::<Vec<_>>()
                        .join(" < ")
                ),
            )
        }
        "tie" => {
            let items = r.data.as_array().unwrap();
            let c: Vec<f64> = items
                .iter()
                .map(|it| {
                    let a = it.as_array().unwrap();
                    t2(
                        ctx,
                        key(a[0].as_str().unwrap()),
                        finger(a[1].as_str().unwrap()),
                        key(a[2].as_str().unwrap()),
                        finger(a[3].as_str().unwrap()),
                    )
                })
                .collect();
            let d = (c[0] - c[1]).abs();
            (
                w * (d - 0.08).max(0.0),
                d <= 0.08,
                format!("tie {:.2} vs {:.2}", c[0], c[1]),
            )
        }
        "infeas" => {
            let a = r.data.as_array().unwrap();
            let c = pt(
                ctx,
                key(a[0].as_str().unwrap()),
                finger(a[1].as_str().unwrap()),
                key(a[2].as_str().unwrap()),
                finger(a[3].as_str().unwrap()),
            );
            (
                w * (8.0 - c).max(0.0),
                c >= 8.0 - 0.01,
                format!("pair cost {:.2} (need >= 8)", c),
            )
        }
        "feasible" => {
            let a = r.data.as_array().unwrap();
            let c = pt(
                ctx,
                key(a[0].as_str().unwrap()),
                finger(a[1].as_str().unwrap()),
                key(a[2].as_str().unwrap()),
                finger(a[3].as_str().unwrap()),
            );
            (
                w * (c - 5.0).max(0.0),
                c < 7.0,
                format!("pair cost {:.2} (need < 7)", c),
            )
        }
        "tiers" => {
            let tiers = r.data.as_array().unwrap();
            let mut tc: Vec<Vec<f64>> = Vec::new();
            for tier in tiers {
                let mut row = Vec::new();
                for it in tier.as_array().unwrap() {
                    let a = it.as_array().unwrap();
                    row.push(t2(
                        ctx,
                        key(a[0].as_str().unwrap()),
                        finger(a[1].as_str().unwrap()),
                        key(a[2].as_str().unwrap()),
                        finger(a[3].as_str().unwrap()),
                    ));
                }
                tc.push(row);
            }
            let mut loss = 0.0;
            let mut ok = true;
            for i in 0..tc.len().saturating_sub(1) {
                let gap = tc[i + 1].iter().cloned().fold(f64::INFINITY, f64::min)
                    - tc[i].iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                loss += (0.02 - gap).max(0.0);
                ok &= gap > 0.0;
            }
            for t in &tc {
                let mx = t.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                let mn = t.iter().cloned().fold(f64::INFINITY, f64::min);
                loss += ((mx - mn) - 0.15).max(0.0);
            }
            let text = tc
                .iter()
                .map(|t| {
                    let mx = t.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                    let mn = t.iter().cloned().fold(f64::INFINITY, f64::min);
                    format!("{:.2}-{:.2}", mn, mx)
                })
                .collect::<Vec<_>>()
                .join(" | ");
            (w * loss, ok, format!("tiers {}", text))
        }
        "cons" => {
            let keys: Vec<&str> = r.data["keys"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_str().unwrap())
                .collect();
            let ids: Vec<usize> = keys.iter().map(|k| key(k)).collect();
            let mut vals = block(ctx, &ids);
            let pm = penalty(ctx, &keys);
            for (i, v) in vals.iter_mut().enumerate() {
                *v += pm[i];
            }
            let n = ids.len();
            let mut reqs: HashMap<usize, HashSet<usize>> = HashMap::new();
            if let Some(arr) = r.data["req"].as_array() {
                for it in arr {
                    let a = it.as_array().unwrap();
                    reqs.entry(a[0].as_u64().unwrap() as usize)
                        .or_default()
                        .insert(finger(a[1].as_str().unwrap()));
                }
            }
            let forbids: HashMap<usize, usize> = r.data["forbid"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .map(|it| {
                            let a = it.as_array().unwrap();
                            (
                                a[0].as_u64().unwrap() as usize,
                                finger(a[1].as_str().unwrap()),
                            )
                        })
                        .collect()
                })
                .unwrap_or_default();
            let margin = r.data["margin"].as_f64().unwrap_or(0.05);
            let mut a_vals = Vec::new();
            let mut b_vals = Vec::new();
            for i in 0..vals.len() {
                let mut fs = vec![0usize; n];
                let mut rem = i;
                for pos in (0..n).rev() {
                    fs[pos] = rem % NF;
                    rem /= NF;
                }
                let mut legal = true;
                for pos in 0..n {
                    if let Some(req) = reqs.get(&pos) {
                        if !req.contains(&fs[pos]) {
                            legal = false;
                            break;
                        }
                    }
                    if let Some(&forbid) = forbids.get(&pos) {
                        if fs[pos] == forbid {
                            legal = false;
                            break;
                        }
                    }
                }
                if vals[i].is_finite() {
                    if legal {
                        a_vals.push(vals[i]);
                    } else {
                        b_vals.push(vals[i]);
                    }
                }
            }
            if a_vals.is_empty() {
                return (
                    5.0 * w,
                    false,
                    format!("{}: required fingering unreachable", keys.join("-")),
                );
            }
            let ca = a_vals.iter().cloned().fold(f64::INFINITY, f64::min);
            if b_vals.is_empty() {
                return (0.0, true, "no alternative".to_string());
            }
            b_vals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let loss: f64 = b_vals
                .iter()
                .take(6)
                .map(|b| (ca - b + margin).max(0.0))
                .sum();
            let ok = if margin >= 0.0 {
                ca - b_vals[0] <= -margin + 1e-9
            } else {
                ca - b_vals[0] <= -margin
            };
            (
                w * loss,
                ok,
                format!(
                    "{} legal {:.2} vs best-other {:.2}",
                    keys.join("-"),
                    ca,
                    b_vals[0]
                ),
            )
        }
        "band" => {
            let item = r.data[0].as_array().unwrap();
            let lo = r.data[1].as_f64().unwrap();
            let hi = r.data[2].as_f64().unwrap();
            let c = pt(
                ctx,
                key(item[0].as_str().unwrap()),
                finger(item[1].as_str().unwrap()),
                key(item[2].as_str().unwrap()),
                finger(item[3].as_str().unwrap()),
            );
            (
                w * ((lo - c).max(0.0) + (c - hi).max(0.0)),
                lo - 0.02 <= c && c <= hi,
                format!("pair cost {:.2} (want {}-{})", c, lo, hi),
            )
        }
        other => (0.0, false, format!("unsupported ruling kind {}", other)),
    }
}

fn probe_rows(ctx: &Context, names: &[&str]) -> Vec<(f64, String)> {
    let ids: Vec<usize> = names.iter().map(|k| key(k)).collect();
    let mut t: Vec<f64> = if ids.len() == 2 {
        ctx.model.bi_many(&ctx.tables.model, &[ids[0]], &[ids[1]])
    } else {
        ctx.model.tri_many(
            &ctx.geom,
            &ctx.tables.model,
            &[ids[0]],
            &[ids[1]],
            &[ids[2]],
        )
    };
    let pm = penalty(ctx, names);
    for (i, v) in t.iter_mut().enumerate() {
        *v += pm[i] + PRESS * ids.len() as f64;
    }
    let mut rows = Vec::new();
    for (i, &c) in t.iter().enumerate() {
        if c.is_finite() {
            let mut fs = vec![0usize; ids.len()];
            let mut rem = i;
            for pos in (0..ids.len()).rev() {
                fs[pos] = rem % NF;
                rem /= NF;
            }
            rows.push((c, fp(&fs)));
        }
    }
    rows.sort_by(|a, b| {
        a.0.partial_cmp(&b.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.1.cmp(&b.1))
    });
    rows
}

fn best(ctx: &Context, names: &[&str]) -> String {
    probe_rows(ctx, names)[0].1.clone()
}

pub fn collect(
    ctx: &Context,
    cfg_name: &str,
    primary_corpus: &[String],
    secondary_corpus: &[String],
    rulings_path: Option<&Path>,
    quiet: bool,
) -> Result<AuditOutcome> {
    let mut lines: Vec<String> = Vec::new();
    macro_rules! out {
        ($e:expr) => {{
            let s: String = $e;
            if !quiet {
                println!("{}", s);
            }
            lines.push(s);
        }};
    }
    out!(format!(
        "config {}  (explicit alt-licence table: {:?})",
        cfg_name,
        {
            let mut v: Vec<(char, char)> = ctx.gates.all.keys().cloned().collect();
            v.sort();
            v
        }
    ));
    out!("=== questionnaire hard probes (physical keys; * = non-default finger) ===".to_string());
    let probes: Vec<(&str, &str)> = vec![
        ("O-L", "OL"),
        ("L-O", "LO"),
        ("K-J-M hey", "KJM"),
        ("F-J-M ney", "FJM"),
        ("M-J yea", "MJ"),
        ("M-J-I yea", "MJI"),
        ("H-T-J exp", "HTJ"),
        ("H-W-J ext", "HWJ"),
        ("I-U-J age", "IUJ"),
        ("N-L-J equ", "NLJ"),
        ("V-G-J ble", "VGJ"),
        ("C-O-R vid", "COR"),
        ("R-O-C div", "ROC"),
        ("K-I", "KI"),
        ("th W-D", "WD"),
        ("ht D-W", "DW"),
        ("as A-S", "AS"),
        ("sa S-A", "SA"),
        ("X-A", "XA"),
        ("A-X", "AX"),
        ("U-J", "UJ"),
        ("U-H", "UH"),
        ("X-W", "XW"),
        ("S-W", "SW"),
        ("W-X-F wxf", "WXF"),
        ("F-X-W fxw", "FXW"),
        ("Q-S", "QS"),
        ("S-Q", "SQ"),
        ("O-K", "OK"),
        ("K-O", "KO"),
        ("K-;", "K;"),
        (";-K", ";K"),
        ("A-E", "AE"),
        ("E-A", "EA"),
        ("T-G", "TG"),
        ("G-T", "GT"),
        ("Y-U", "YU"),
        ("U-Y", "UY"),
        ("D-A", "DA"),
        ("A-D", "AD"),
        ("F-S", "FS"),
        ("S-F", "SF"),
        ("O-J", "OJ"),
        ("J-O", "JO"),
    ];
    let mut top: HashMap<String, String> = HashMap::new();
    for (_name, seq) in &probes {
        let chars: Vec<&str> = seq
            .chars()
            .map(|c| {
                // each character is a physical key label
                let mut buf = [0u8; 4];
                let s: &str = c.encode_utf8(&mut buf);
                Box::leak(s.to_string().into_boxed_str()) as &str
            })
            .collect();
        let rows = probe_rows(ctx, &chars);
        out!(format!("  {}:", seq));
        for (c, fs) in rows.iter().take(5) {
            let detail: Vec<String> = chars
                .iter()
                .zip(fs.chars())
                .map(|(k, f)| {
                    let star = if f.to_string() != ctx.model.default_finger[key(k)].to_string() {
                        "*"
                    } else {
                        ""
                    };
                    format!("{}:{}{}", k, f, star)
                })
                .collect();
            out!(format!("    {:8.3}  {}", c, detail.join(" -> ")));
        }
        top.insert(seq.to_string(), rows[0].1.clone());
    }
    let mut checks: Vec<(String, bool, String)> = Vec::new();
    out!(String::new());
    out!("=== hard checks ===".to_string());
    macro_rules! chk {
        ($name:expr, $ok:expr, $detail:expr) => {{
            let name: &str = $name;
            let ok: bool = $ok;
            let detail: String = $detail;
            checks.push((name.to_string(), ok, detail.clone()));
            let s = format!(
                "  [{}] {}  {}",
                if ok { "PASS" } else { "FAIL" },
                name,
                detail
            );
            if !quiet {
                println!("{}", s);
            }
            lines.push(s);
        }};
    }
    let repeated: Vec<char> = "QWERTYUIOPASDFGHJKLZXCVBNM"
        .chars()
        .filter(|c| {
            let s = c.to_string();
            let rows = probe_rows(ctx, &[&s, &s]);
            rows[0].1.chars().next() != rows[0].1.chars().nth(1)
        })
        .collect();
    chk!(
        "repeated keys use one finger",
        repeated.is_empty(),
        if repeated.is_empty() {
            "26/26".into()
        } else {
            format!("bad: {:?}", repeated)
        }
    );
    chk!("O->L has L index", top["OL"] == "MI", top["OL"].clone());
    chk!(
        "K-J-M = ring/middle/index",
        top["KJM"] == "RMI",
        top["KJM"].clone()
    );
    chk!(
        "F-J-M = index/middle/index",
        top["FJM"] == "IMI",
        top["FJM"].clone()
    );
    let lo = probe_rows(ctx, &["L", "O"]);
    let best_i = lo
        .iter()
        .filter(|(_, f)| f.starts_with('I'))
        .map(|(c, _)| *c)
        .fold(f64::INFINITY, f64::min);
    chk!(
        "L->O keeps L index within 0.2",
        best_i < lo[0].0 + 0.2,
        format!(
            "top {} / best L-index {:.3} vs {:.3}",
            lo[0].1, best_i, lo[0].0
        )
    );
    chk!(
        "M-J-I first two strokes index",
        top["MJI"][..2] == *"II",
        top["MJI"].clone()
    );
    chk!(
        "H-T-J blocks J middle",
        top["HTJ"].chars().nth(2) != Some('M'),
        top["HTJ"].clone()
    );
    chk!(
        "I-U-J blocks J middle",
        top["IUJ"].chars().nth(2) != Some('M'),
        top["IUJ"].clone()
    );
    chk!(
        "V-G-J forces V/G index",
        top["VGJ"][..2] == *"II",
        top["VGJ"].clone()
    );
    chk!(
        "C-O-R endpoints index",
        top["COR"].starts_with('I') && top["COR"].chars().nth(2) == Some('I'),
        top["COR"].clone()
    );
    chk!(
        "R-O-C endpoints index",
        top["ROC"].starts_with('I') && top["ROC"].chars().nth(2) == Some('I'),
        top["ROC"].clone()
    );
    let lg: Vec<&str> = KEYS
        .iter()
        .filter(|&&k| k != "L" && k != "O")
        .filter(|&&k| best(ctx, &["L", k]).starts_with('I'))
        .cloned()
        .collect();
    chk!(
        "L index only with O key (bigram)",
        lg.is_empty(),
        if lg.is_empty() {
            "all non-O neighbours".into()
        } else {
            format!("bad: {:?}", lg)
        }
    );
    out!(String::new());
    out!("=== 2026-10-27 answers ===".to_string());
    chk!(
        "W->D ring/middle (th)",
        top["WD"] == "RM",
        top["WD"].clone()
    );
    let dw = probe_rows(ctx, &["D", "W"]);
    chk!(
        "D->W middle/ring is cheaper than W->D",
        dw[0].0 < probe_rows(ctx, &["W", "D"])[0].0 && top["DW"] == "MR",
        top["DW"].clone()
    );
    chk!(
        "A->S pinky/ring (2026-10-29 correction)",
        top["AS"] == "PR",
        top["AS"].clone()
    );
    chk!("S->A ring/pinky", top["SA"] == "RP", top["SA"].clone());
    let xa = probe_rows(ctx, &["X", "A"]);
    let xa_map: HashMap<String, f64> = xa.iter().map(|(c, f)| (f.clone(), *c)).collect();
    chk!(
        "X->A index/ring first and cheaper than middle/ring; ring X + pinky A impossible",
        top["XA"] == "IR"
            && top["AX"] == "RI"
            && xa_map["IR"] < xa_map["MR"]
            && xa
                .iter()
                .filter(|(_, f)| f == "RP")
                .map(|(c, _)| *c)
                .fold(f64::INFINITY, f64::min)
                >= 8.0,
        format!("{} {}", top["XA"], top["AX"])
    );
    chk!(
        "Q-S = middle/index, S-Q = index/middle",
        top["QS"] == "MI" && top["SQ"] == "IM",
        format!("{} {}", top["QS"], top["SQ"])
    );
    chk!(
        "O->K ring/middle, K->O middle/ring",
        top["OK"] == "RM" && top["KO"] == "MR",
        format!("{} {}", top["OK"], top["KO"])
    );
    chk!(
        "K->; middle/pinky, ;->K pinky/middle",
        top["K;"] == "MP" && top[";K"] == "PM",
        format!("{} {}", top["K;"], top[";K"])
    );
    chk!(
        "A->E pinky/middle, E->A middle/pinky",
        top["AE"] == "PM" && top["EA"] == "MP",
        format!("{} {}", top["AE"], top["EA"])
    );
    let rest = [
        [("A", "P"), ("W", "R"), ("E", "M"), ("F", "I")],
        [(";", "P"), ("O", "R"), ("I", "M"), ("J", "I")],
    ];
    let mut bad_rest = Vec::new();
    for hand in &rest {
        for (a, fa) in hand {
            for (b, fb) in hand {
                if a == b {
                    continue;
                }
                if best(ctx, &[a, b]) != format!("{}{}", fa, fb) {
                    bad_rest.push(format!("{}{}", a, b));
                }
            }
        }
    }
    chk!(
        "defaults first: both keys at rest -> rest fingers (24 pairs)",
        bad_rest.is_empty(),
        if bad_rest.is_empty() {
            "24/24".into()
        } else {
            format!("bad: {:?}", bad_rest)
        }
    );
    chk!(
        "U->J, U->H middle on U",
        top["UJ"] == "MI" && top["UH"] == "MI",
        format!("{} {}", top["UJ"], top["UH"])
    );
    chk!(
        "X->W and S->W index/middle",
        top["XW"] == "IM" && top["SW"] == "IM",
        format!("{} {}", top["XW"], top["SW"])
    );
    chk!("wxf = MII", top["WXF"] == "MII", top["WXF"].clone());
    chk!(
        "fxw does not use the M/R full-span scissor",
        top["FXW"] != "IMR",
        top["FXW"].clone()
    );
    chk!(
        "T->G middle/index, G->T index/middle",
        top["TG"] == "MI" && top["GT"] == "IM",
        format!("{} {}", top["TG"], top["GT"])
    );
    chk!(
        "YU and RT/TR ring-middle / middle-ring on both hands",
        top["YU"] == "MR"
            && top["UY"] == "RM"
            && best(ctx, &["R", "T"]) == "RM"
            && best(ctx, &["T", "R"]) == "MR",
        format!(
            "{} {} {} {}",
            top["YU"],
            top["UY"],
            best(ctx, &["R", "T"]),
            best(ctx, &["T", "R"])
        )
    );
    chk!(
        "D->A middle/pinky, A->D pinky/middle",
        top["DA"] == "MP" && top["AD"] == "PM",
        format!("{} {}", top["DA"], top["AD"])
    );
    chk!(
        "F->S index/ring, S->F ring/index; O->J ring/index, J->O index/ring",
        top["FS"] == "IR" && top["SF"] == "RI" && top["OJ"] == "RI" && top["JO"] == "IR",
        ["FS", "SF", "OJ", "JO"]
            .iter()
            .map(|k| top[*k].clone())
            .collect::<Vec<_>>()
            .join(" ")
    );
    chk!("X->A order: index/ring < middle/ring < index/middle < any pinky fingering; index/pinky never", xa_map.get("IP").copied().unwrap_or(99.0) >= 8.0 && xa_map["IR"] < xa_map["MR"] && xa_map["MR"] < xa_map["IM"] && xa_map["IM"] < xa_map["MP"], format!("IR {:.2} MR {:.2} IM {:.2} MP {:.2} IP {:.2}", xa_map["IR"], xa_map["MR"], xa_map["IM"], xa_map["MP"], xa_map.get("IP").copied().unwrap_or(99.0)));
    let mut ipbad = Vec::new();
    for a in KEYS {
        for b in KEYS {
            if a == b {
                continue;
            }
            let a_left = "QWERTASDFGZXCVB".contains(a.chars().next().unwrap());
            let b_left = "QWERTASDFGZXCVB".contains(b.chars().next().unwrap());
            if a_left != b_left {
                continue;
            }
            let ka = key(a);
            let kb = key(b);
            let pf = pair_feature(&ctx.geom, ka, finger("P"), kb, finger("I"));
            if pf.s < 1.9 && pt(ctx, ka, finger("P"), kb, finger("I")) < 8.0 {
                ipbad.push(format!("{}{}", a, b));
            }
        }
    }
    chk!(
        "index + pinky needs >= 1.9 key widths between the keys (pattern)",
        ipbad.is_empty(),
        if ipbad.is_empty() {
            "all such pairs impossible".into()
        } else {
            format!("bad: {:?}", &ipbad[..ipbad.len().min(5)])
        }
    );
    chk!(
        "2026-10-29: ;->U PM, U->; MP; T->F IM, F->T MI; ;->L PR, L->; RP",
        best(ctx, &[";", "U"]) == "PM"
            && best(ctx, &["U", ";"]) == "MP"
            && best(ctx, &["T", "F"]) == "IM"
            && best(ctx, &["F", "T"]) == "MI"
            && best(ctx, &[";", "L"]) == "PR"
            && best(ctx, &["L", ";"]) == "RP",
        [
            (";", "U"),
            ("U", ";"),
            ("T", "F"),
            ("F", "T"),
            (";", "L"),
            ("L", ";")
        ]
        .iter()
        .map(|(a, b)| best(ctx, &[a, b]))
        .collect::<Vec<_>>()
        .join(" ")
    );
    let cu = probe_rows(ctx, &[";", "U"]);
    let cu2 = probe_rows(ctx, &["U", ";"]);
    let cu_map: HashMap<String, f64> = cu.iter().map(|(c, f)| (f.clone(), *c)).collect();
    let cu2_map: HashMap<String, f64> = cu2.iter().map(|(c, f)| (f.clone(), *c)).collect();
    chk!(
        ";-U with ring + middle is borderline impossible (>= 5)",
        cu_map["RM"] >= 5.0 && cu2_map["MR"] >= 5.0,
        format!("{:.2} {:.2}", cu_map["RM"], cu2_map["MR"])
    );
    chk!(
        "A->S PR and the ring/pinky pair is not scissor-penalised away (S->A RP)",
        best(ctx, &["A", "S"]) == "PR" && best(ctx, &["S", "A"]) == "RP",
        format!("{} {}", best(ctx, &["A", "S"]), best(ctx, &["S", "A"]))
    );
    let mut ke_bad = Vec::new();
    for (i, k) in KEYS.iter().enumerate() {
        let e = &ctx.tables.model.e[i * NF..(i + 1) * NF];
        let mut best_f = 0;
        let mut best_v = f64::INFINITY;
        for f in 0..NF {
            if e[f] < best_v {
                best_v = e[f];
                best_f = f;
            }
        }
        if FINGERS[best_f] != ctx.model.default_finger[i] {
            ke_bad.push(*k);
        }
    }
    chk!(
        "isolated key -> your default fingermap (all 30 keys + ;)",
        ke_bad.is_empty(),
        if ke_bad.is_empty() {
            "all keys".into()
        } else {
            format!("bad: {:?}", ke_bad)
        }
    );
    let mut asym = Vec::new();
    for a in KEYS {
        for b in KEYS {
            if a == b {
                continue;
            }
            for fa in 0..NF {
                for fb in 0..NF {
                    if fa == fb {
                        continue;
                    }
                    let x = t2(ctx, key(a), fa, key(b), fb);
                    let y = t2(ctx, key(b), fb, key(a), fa);
                    if x.is_finite() && y.is_finite() && (x - y).abs() > 0.02 {
                        let mut pair = [FINGERS[fa], FINGERS[fb]];
                        pair.sort();
                        let ps: String = pair.iter().collect();
                        if ps != "MR" && ps != "PR" {
                            asym.push(format!("{}{}", a, b));
                        }
                    }
                }
            }
        }
    }
    chk!(
        "A->B and B->A price alike except ring/middle and pinky/ring pairs (direction rules)",
        asym.is_empty(),
        if asym.is_empty() {
            "symmetric".into()
        } else {
            format!(
                "{} asymmetric, e.g. {:?}",
                asym.len(),
                &asym[..asym.len().min(3)]
            )
        }
    );
    out!(format!(
        "probes+checks: {}/{}",
        checks.iter().filter(|(_, ok, _)| *ok).count(),
        checks.len()
    ));

    let primary_entries: Vec<(String, String)> = if primary_corpus.is_empty() {
        ctx.cfg
            .primary_corpus
            .iter()
            .map(|e| {
                let p = crate::context::corpus_path(&ctx.cfg, e);
                (e.label(), p.display().to_string())
            })
            .collect()
    } else {
        primary_corpus
            .iter()
            .map(|p| (p.clone(), p.clone()))
            .collect()
    };
    let secondary_entries: Vec<(String, String)> = if secondary_corpus.is_empty() {
        ctx.cfg
            .secondary_corpus
            .iter()
            .map(|e| {
                let p = crate::context::corpus_path(&ctx.cfg, e);
                (e.label(), p.display().to_string())
            })
            .collect()
    } else {
        secondary_corpus
            .iter()
            .map(|p| (p.clone(), p.clone()))
            .collect()
    };
    let mut corpus_info = serde_json::Map::new();
    for (label, entries) in [
        ("primary", &primary_entries),
        ("secondary", &secondary_entries),
    ] {
        for (name, path) in entries {
            let corpus = crate::corpus::load_many(&[std::path::PathBuf::from(path)])?;
            corpus_info.insert(
                path.clone(),
                json!({
                    "role": label,
                    "letters": corpus.total_letters.round_ties_even() as i64,
                    "bigrams": corpus.total_bigrams.round_ties_even() as i64,
                    "trigrams": corpus.total_trigrams.round_ties_even() as i64,
                }),
            );
            out!(format!(
                "corpus {} {}: letters={} bigrams={} trigrams={}",
                label,
                path,
                fmt_thousands(corpus.total_letters as u64),
                fmt_thousands(corpus.total_bigrams as u64),
                fmt_thousands(corpus.total_trigrams as u64)
            ));
            let _ = name;
        }
    }

    out!(String::new());
    out!("=== every ruling (legacy + round 8) ===".to_string());
    let rulings = load_rulings(rulings_path)?;
    let mut passed = 0usize;
    for r in &rulings {
        let (_loss, ok, text) = eval_ruling(ctx, r);
        if ok {
            passed += 1;
        }
        out!(format!(
            "  [{}] {:<16} {}",
            if ok { "PASS" } else { "FAIL" },
            r.id,
            text
        ));
    }
    out!(format!("rulings: {}/{}", passed, rulings.len()));
    let mut anchors = Vec::new();
    if let Some((nb, total)) = anchor_loss(ctx)? {
        let text = format!(
            "{}/{} default-finger pairs more than 0.25 away from the shipped price",
            nb, total
        );
        out!(format!("anchor (not a ruling): {}", text));
        anchors.push(text);
    }
    Ok(AuditOutcome {
        rulings_passed: passed,
        rulings_total: rulings.len(),
        probes_passed: checks.iter().filter(|(_, ok, _)| *ok).count(),
        probes_total: checks.len(),
        corpora: Value::Object(corpus_info),
        anchors,
        lines,
    })
}

fn fmt_thousands(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

fn anchor_loss(ctx: &Context) -> Result<Option<(usize, usize)>> {
    let path = crate::paths::model_dir().join("anchor_pairs.json");
    if !path.is_file() {
        return Ok(None);
    }
    let rows: Vec<Value> = serde_json::from_str(&crate::paths::read_to_string(&path)?)?;
    let mut total = 0usize;
    let mut bad = 0usize;
    for row in rows {
        let a = row[0].as_str().unwrap();
        let fa = row[1].as_str().unwrap();
        let b = row[2].as_str().unwrap();
        let fb = row[3].as_str().unwrap();
        // default fingers the shipped model itself does not follow
        if ["R", "T", "Y", "X", "."].iter().any(|k| *k == a || *k == b) {
            continue;
        }
        let (a, b) = (a.chars().next().unwrap(), b.chars().next().unwrap());
        let (fa, fb) = (fa.chars().next().unwrap(), fb.chars().next().unwrap());
        // ring on the bottom row + pinky on the home row is impossible by ruling
        if ((fa == 'R' && fb == 'P') || (fa == 'P' && fb == 'R'))
            && ((fa == 'R' && "X,.".contains(a) && "A;".contains(b))
                || (fb == 'R' && "X,.".contains(b) && "A;".contains(a)))
        {
            continue;
        }
        total += 1;
        let c = pt(
            ctx,
            key(&a.to_string()),
            finger(&fa.to_string()),
            key(&b.to_string()),
            finger(&fb.to_string()),
        );
        let expected = row[4].as_f64().unwrap_or(0.0);
        let d = if c.is_finite() { c - expected } else { 10.0 };
        if (d.abs() - 0.25).max(0.0) > 0.0 {
            bad += 1;
        }
    }
    Ok(Some((bad, total)))
}

pub struct AuditArgs<'a> {
    pub config_name: Option<&'a str>,
    pub config_file: Option<&'a Path>,
    pub primary_corpus: &'a [String],
    pub secondary_corpus: &'a [String],
    pub rulings: Option<&'a Path>,
    pub out: Option<&'a Path>,
    pub out_json: Option<&'a Path>,
}

pub fn run(args: AuditArgs) -> Result<()> {
    let ctx = Context::load(args.config_file)?;
    let cfg_label = match args.config_name {
        Some(name) => name.to_string(),
        None => args
            .config_file
            .map(Path::to_path_buf)
            .unwrap_or_else(paths::config_file)
            .display()
            .to_string(),
    };
    let outcome = collect(
        &ctx,
        &cfg_label,
        args.primary_corpus,
        args.secondary_corpus,
        args.rulings,
        false,
    )?;
    let out_path = args
        .out
        .map(Path::to_path_buf)
        .unwrap_or_else(|| paths::result_path("audit", args.config_name, ".txt"));
    let out_json = args
        .out_json
        .map(Path::to_path_buf)
        .unwrap_or_else(|| paths::result_path("audit", args.config_name, ".json"));
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&out_path, outcome.lines.join("\n") + "\n")?;
    if let Some(parent) = out_json.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let payload = json!({
        "config": args.config_name,
        "probe_checks": {"passed": outcome.probes_passed, "total": outcome.probes_total},
        "rulings": {"passed": outcome.rulings_passed, "total": outcome.rulings_total},
        "corpora": outcome.corpora,
        "anchors": outcome.anchors,
    });
    std::fs::write(&out_json, serde_json::to_string_pretty(&payload)? + "\n")?;
    Ok(())
}
