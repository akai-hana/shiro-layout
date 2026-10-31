//! `analyze`: detailed layout report.
//!
//! Port of `shirolyzer/commands/analyze.py`.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{anyhow, Result};
use serde_json::json;

use crate::context;
use crate::evaluate::score_layout;
use crate::geometry::{finger_index, key_index, KEYS};
use crate::model::pair_feature;
use crate::paths;
use crate::scorer::Tables;

const LETTERS: &str = "abcdefghijklmnopqrstuvwxyz";
const FID2: [char; 5] = ['P', 'R', 'M', 'I', 'T'];
const FORDER: [usize; 4] = [0, 1, 2, 3];

type KeyMap = [Option<usize>; 27];

fn letter_id(ch: char) -> Option<usize> {
    match ch {
        'a'..='z' => Some(ch as usize - 'a' as usize),
        ' ' => Some(crate::corpus::SPACE_INDEX),
        _ => None,
    }
}

fn id_char(id: usize) -> char {
    if id == crate::corpus::SPACE_INDEX {
        ' '
    } else {
        (b'a' + id as u8) as char
    }
}

fn keymap(layout: &str) -> KeyMap {
    let mut map = [None; 27];
    for (i, ch) in layout.chars().enumerate() {
        if let (Some(id), Some(key)) = (letter_id(ch), key_index(KEYS[i])) {
            map[id] = Some(key);
        }
    }
    map
}

fn decode_finger(value: u8) -> char {
    FID2[(value / 5) as usize]
}

fn decode_finger_at(value: [u8; 3], pos: usize) -> char {
    FID2[value[pos] as usize]
}

pub struct CategoryReport {
    pub cats: HashMap<String, f64>,
    pub worst: Vec<(f64, String, char, char, f64)>,
    pub scissors: Vec<(f64, f64, String, char, char, f64, String)>,
}

pub fn category_report(
    layout: &str,
    corpus: &crate::corpus::Corpus,
    tables: &Tables,
    geom: &crate::geometry::Geometry,
) -> CategoryReport {
    let m = keymap(layout);
    let mut stats: HashMap<String, f64> = HashMap::new();
    let mut total = 0.0;
    let mut worst = Vec::new();
    let mut scissors = Vec::new();
    for (ids, &cnt) in corpus.bi_ids.iter().zip(&corpus.bi_counts) {
        let (Some(i), Some(j)) = (m[ids[0]], m[ids[1]]) else {
            continue;
        };
        let (a, b) = (id_char(ids[0]), id_char(ids[1]));
        total += cnt;
        let cost = tables.big[crate::scorer::big_idx(i, j)];
        let enc = tables.big_f[crate::scorer::big_idx(i, j)];
        let f1 = decode_finger(enc);
        let f2 = FID2[(enc % 5) as usize];
        worst.push((cost, format!("{}{}", a, b), f1, f2, cnt));
        if i == j {
            *stats.entry("repeat".into()).or_insert(0.0) += cnt;
        } else if geom.hand[i] != geom.hand[j] {
            *stats.entry("alt".into()).or_insert(0.0) += cnt;
        } else if f1 == f2 {
            *stats.entry("sfb".into()).or_insert(0.0) += cnt;
        } else {
            let d1 = FORDER[finger_index(f1).unwrap()];
            let d2 = FORDER[finger_index(f2).unwrap()];
            let dcol = (geom.cx[i] - geom.cx[j]).abs();
            if d2.abs_diff(d1) >= 2 || dcol >= 3.0 {
                *stats.entry("leap".into()).or_insert(0.0) += cnt;
            } else if d2 > d1 {
                *stats.entry("roll_in".into()).or_insert(0.0) += cnt;
            } else {
                *stats.entry("roll_out".into()).or_insert(0.0) += cnt;
            }
            let pf = pair_feature(
                geom,
                i,
                finger_index(f1).unwrap(),
                j,
                finger_index(f2).unwrap(),
            );
            if pf.g > 0.25 {
                let (outer, inner) = if d1 < d2 { (f1, f2) } else { (f2, f1) };
                let kind = match inner {
                    'M' => Some("scissor_IM"),
                    'R' => Some("scissor_MR"),
                    'P' => Some("scissor_RP"),
                    _ => None,
                };
                if let Some(kind) = kind {
                    let suffix = if geom.row[i].abs_diff(geom.row[j]) >= 2 {
                        "_fsb"
                    } else {
                        "_hsb"
                    };
                    *stats.entry(format!("{}{}", kind, suffix)).or_insert(0.0) += cnt;
                    scissors.push((
                        cost * cnt,
                        cost,
                        format!("{}{}", a, b),
                        f1,
                        f2,
                        cnt,
                        format!("{}{}", outer, inner),
                    ));
                }
            }
        }
    }
    if total == 0.0 {
        total = 1.0;
    }
    let mut cats = HashMap::new();
    for key in [
        "sfb",
        "scissor_IM_hsb",
        "scissor_IM_fsb",
        "scissor_MR",
        "scissor_RP",
        "roll_in",
        "roll_out",
        "leap",
        "alt",
        "repeat",
    ] {
        let value = stats.get(key).copied().unwrap_or(0.0);
        cats.insert(key.to_string(), value / total * 100.0);
    }
    cats.insert(
        "hand_alternation".to_string(),
        *cats.get("alt").unwrap_or(&0.0),
    );
    cats.insert("total_tokens".to_string(), total);
    worst.sort_by(|a, b| {
        b.0.partial_cmp(&a.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.1.cmp(&a.1))
    });
    scissors.sort_by(|a, b| {
        b.0.partial_cmp(&a.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal))
    });
    worst.truncate(30);
    scissors.truncate(20);
    CategoryReport {
        cats,
        worst,
        scissors,
    }
}

pub fn trigram_patterns(
    layout: &str,
    corpus: &crate::corpus::Corpus,
    tables: &Tables,
    geom: &crate::geometry::Geometry,
) -> HashMap<String, f64> {
    let m = keymap(layout);
    let mut acc: HashMap<String, f64> = HashMap::new();
    let mut total = 0.0;
    for (letter_ids, &cnt) in corpus.tri_ids.iter().zip(&corpus.tri_counts) {
        let (Some(k0), Some(k1), Some(k2)) = (m[letter_ids[0]], m[letter_ids[1]], m[letter_ids[2]])
        else {
            continue;
        };
        let ids = [k0, k1, k2];
        let enc = tables.tri_f[crate::scorer::tri_idx(ids[0], ids[1], ids[2])];
        let fs = [
            decode_finger_at(enc, 0),
            decode_finger_at(enc, 1),
            decode_finger_at(enc, 2),
        ];
        let hs = [geom.hand[ids[0]], geom.hand[ids[1]], geom.hand[ids[2]]];
        let os_ = [
            FORDER[finger_index(fs[0]).unwrap()],
            FORDER[finger_index(fs[1]).unwrap()],
            FORDER[finger_index(fs[2]).unwrap()],
        ];
        total += cnt;
        let add = |acc: &mut HashMap<String, f64>, key: &str| {
            *acc.entry(key.to_string()).or_insert(0.0) += cnt;
        };
        let has_repeat = ids[0] == ids[1] || ids[1] == ids[2] || ids[0] == ids[2];
        let is_sft = if hs[0] == hs[1] && hs[1] == hs[2] {
            fs[0] == fs[1] || fs[1] == fs[2]
        } else if hs[0] == hs[1] {
            fs[0] == fs[1]
        } else if hs[1] == hs[2] {
            fs[1] == fs[2]
        } else {
            false
        };
        if is_sft && has_repeat {
            add(&mut acc, "repeat");
            continue;
        }
        if (hs == [0, 1, 0]) || (hs == [1, 0, 1]) {
            add(&mut acc, "alternate");
        } else if hs[0] == hs[1] && hs[1] == hs[2] {
            if fs[0] == fs[1] || fs[1] == fs[2] {
                add(&mut acc, "sfb_tri");
            } else if (os_[1] as i64 - os_[0] as i64) * (os_[2] as i64 - os_[1] as i64) < 0 {
                if fs.contains(&'I') {
                    add(&mut acc, "redirect");
                } else {
                    add(&mut acc, "redirect_weak");
                }
            } else {
                add(&mut acc, "onehand");
            }
        } else if hs[0] == hs[1] {
            if fs[0] == fs[1] {
                add(&mut acc, "sfb_tri");
            } else if os_[0] < os_[1] {
                add(&mut acc, "inroll");
            } else {
                add(&mut acc, "outroll");
            }
        } else if hs[1] == hs[2] {
            if fs[1] == fs[2] {
                add(&mut acc, "sfb_tri");
            } else if os_[1] < os_[2] {
                add(&mut acc, "inroll");
            } else {
                add(&mut acc, "outroll");
            }
        } else {
            add(&mut acc, "other");
        }
    }
    if total == 0.0 {
        total = 1.0;
    }
    let mut out: HashMap<String, f64> = acc
        .into_iter()
        .map(|(k, v)| (k, v / total * 100.0))
        .collect();
    let rolls =
        out.get("inroll").copied().unwrap_or(0.0) + out.get("outroll").copied().unwrap_or(0.0);
    let redirects = out.get("redirect").copied().unwrap_or(0.0)
        + out.get("redirect_weak").copied().unwrap_or(0.0);
    let alternation = out.get("alternate").copied().unwrap_or(0.0);
    out.insert("total_rolls".into(), rolls);
    out.insert("total_redirects".into(), redirects);
    out.insert("hand_alternation".into(), alternation);
    out
}

pub fn finger_usage(
    layout: &str,
    corpus: &crate::corpus::Corpus,
    tables: &Tables,
    geom: &crate::geometry::Geometry,
) -> HashMap<String, f64> {
    let m = keymap(layout);
    let mut usage = [[0.0f64; 4]; 2];
    for (letter_ids, &cnt) in corpus.tri_ids.iter().zip(&corpus.tri_counts) {
        let (Some(k0), Some(k1), Some(k2)) = (m[letter_ids[0]], m[letter_ids[1]], m[letter_ids[2]])
        else {
            continue;
        };
        let ids = [k0, k1, k2];
        let enc = tables.tri_f[crate::scorer::tri_idx(ids[0], ids[1], ids[2])];
        for (pos, &k) in ids.iter().enumerate() {
            let f = decode_finger_at(enc, pos);
            usage[geom.hand[k]][finger_index(f).unwrap()] += cnt;
        }
    }
    let total: f64 = usage.iter().flat_map(|h| h.iter()).sum::<f64>().max(1.0);
    let mut out = HashMap::new();
    for (h, label) in [(0, 'L'), (1, 'R')] {
        for f in ['P', 'R', 'M', 'I'] {
            out.insert(
                format!("{}{}", label, f),
                usage[h][finger_index(f).unwrap()] / total * 100.0,
            );
        }
    }
    out.insert("handL".into(), usage[0].iter().sum::<f64>() / total * 100.0);
    out.insert("handR".into(), usage[1].iter().sum::<f64>() / total * 100.0);
    out
}

pub struct GuideEntry {
    pub key: String,
    pub default: char,
    pub share: Vec<(char, f64)>,
    pub alt_contexts: Vec<(char, Vec<(String, f64)>)>,
}

pub fn fingering_guide(
    layout: &str,
    corpus: &crate::corpus::Corpus,
    tables: &Tables,
    model: &crate::model::Model,
) -> HashMap<char, GuideEntry> {
    let m = keymap(layout);
    let mut agg: HashMap<char, HashMap<char, f64>> = HashMap::new();
    let mut ctx: HashMap<char, HashMap<char, HashMap<[char; 3], f64>>> = HashMap::new();
    for (letter_ids, &cnt) in corpus.tri_ids.iter().zip(&corpus.tri_counts) {
        let (Some(k0), Some(k1), Some(k2)) = (m[letter_ids[0]], m[letter_ids[1]], m[letter_ids[2]])
        else {
            continue;
        };
        let ids = [k0, k1, k2];
        let enc = tables.tri_f[crate::scorer::tri_idx(ids[0], ids[1], ids[2])];
        let tg = [
            id_char(letter_ids[0]),
            id_char(letter_ids[1]),
            id_char(letter_ids[2]),
        ];
        for (pos, &ch) in [tg[0], tg[1], tg[2]].iter().enumerate() {
            let f = decode_finger_at(enc, pos);
            *agg.entry(ch).or_default().entry(f).or_insert(0.0) += cnt;
            *ctx.entry(ch)
                .or_default()
                .entry(f)
                .or_default()
                .entry(tg)
                .or_insert(0.0) += cnt;
        }
    }
    let mut guide = HashMap::new();
    for ch in LETTERS.chars() {
        let Some(key) = m[letter_id(ch).unwrap()] else {
            continue;
        };
        let default = model.default_finger[key];
        let entry = agg.get(&ch).cloned().unwrap_or_default();
        let total: f64 = entry.values().sum::<f64>().max(1.0);
        let mut share: Vec<(char, f64)> =
            entry.iter().map(|(f, c)| (*f, c / total * 100.0)).collect();
        share.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.0.cmp(&b.0))
        });
        let mut alt_contexts = Vec::new();
        for &(f, _) in &share {
            if f == default {
                continue;
            }
            let mut items: Vec<([char; 3], f64)> = ctx
                .get(&ch)
                .and_then(|m| m.get(&f))
                .map(|m| m.iter().map(|(k, v)| (*k, *v)).collect())
                .unwrap_or_default();
            items.sort_by(|a, b| {
                b.1.partial_cmp(&a.1)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| a.0.cmp(&b.0))
            });
            items.truncate(8);
            alt_contexts.push((
                f,
                items
                    .into_iter()
                    .map(|(tg, n)| (tg.iter().collect(), n))
                    .collect(),
            ));
        }
        guide.insert(
            ch,
            GuideEntry {
                key: KEYS[key].to_string(),
                default,
                share,
                alt_contexts,
            },
        );
    }
    guide
}

pub struct AnalyzeArgs<'a> {
    pub layout: Option<&'a str>,
    pub config: Option<&'a Path>,
    pub name: Option<&'a str>,
    pub primary_corpus: &'a [String],
    pub secondary_corpus: &'a [String],
    pub out: Option<&'a Path>,
    pub out_json: Option<&'a Path>,
}

pub fn run(args: AnalyzeArgs) -> Result<()> {
    let ctx = context::Context::load(args.config)?;
    let latest;
    let layout = match args.layout {
        Some(layout) => layout,
        None => {
            latest = paths::read_latest_layout()?;
            &latest
        }
    };
    if layout.chars().count() != 30 {
        return Err(anyhow!("layout must be 30 characters"));
    }
    let out_path = args
        .out
        .map(Path::to_path_buf)
        .unwrap_or_else(|| paths::result_path("analysis", args.name, ".txt"));
    let out_json = args
        .out_json
        .map(Path::to_path_buf)
        .unwrap_or_else(|| paths::result_path("analysis", args.name, ".json"));

    let (corpus, primary_paths) = if !args.primary_corpus.is_empty() {
        (
            context::load_paths(args.primary_corpus)?,
            args.primary_corpus.to_vec(),
        )
    } else {
        context::load_role(&ctx.cfg, "primary")?
    };
    let (secondary_corpus, secondary_paths) = if !args.secondary_corpus.is_empty() {
        (
            Some(context::load_paths(args.secondary_corpus)?),
            args.secondary_corpus.to_vec(),
        )
    } else {
        match context::load_role(&ctx.cfg, "secondary") {
            Ok((corpus, labels)) => (Some(corpus), labels),
            Err(_) => (None, Vec::new()),
        }
    };

    let (report, (pats, (use_, guide))) = rayon::join(
        || category_report(layout, &corpus, &ctx.tables, &ctx.geom),
        || {
            rayon::join(
                || trigram_patterns(layout, &corpus, &ctx.tables, &ctx.geom),
                || {
                    rayon::join(
                        || finger_usage(layout, &corpus, &ctx.tables, &ctx.geom),
                        || fingering_guide(layout, &corpus, &ctx.tables, &ctx.model),
                    )
                },
            )
        },
    );
    let primary_score = score_layout(layout, &corpus, &ctx.tables);
    let secondary_score = secondary_corpus
        .as_ref()
        .map(|c| score_layout(layout, c, &ctx.tables));

    let mut lines = Vec::new();
    let mut out = |s: String| {
        println!("{}", s);
        lines.push(s);
    };
    out(format!("layout: {}", layout));
    out("grid (QWERTY order, left/right split 5+5):".to_string());
    let chars: Vec<char> = layout.chars().collect();
    for r in 0..3 {
        let row: String = chars[r * 10..(r + 1) * 10].iter().collect();
        out(format!("  {} | {}", &row[..5], &row[5..]));
    }
    out(format!(
        "primary={:.9}  corpus={}",
        primary_score,
        primary_paths.join(",")
    ));
    if let Some(score) = secondary_score {
        out(format!(
            "secondary={:.9}  corpus={}",
            score,
            secondary_paths.join(",")
        ));
    }
    out(String::new());
    out("== bigram categories (% of letter bigrams, chosen fingering) ==".to_string());
    for k in [
        "sfb",
        "scissor_IM_hsb",
        "scissor_IM_fsb",
        "scissor_MR",
        "scissor_RP",
        "roll_in",
        "roll_out",
        "leap",
        "alt",
        "hand_alternation",
        "repeat",
    ] {
        out(format!(
            "  {:<16} {:6.2}",
            k,
            report.cats.get(k).copied().unwrap_or(0.0)
        ));
    }
    out(String::new());
    out("== worst bigrams (chosen cost, fingering, count) ==".to_string());
    for (cost, bg, f1, f2, cnt) in report.worst.iter().take(20) {
        out(format!(
            "  {}  n={:8.0}  {:6.2}  {}{}",
            bg, cnt, cost, f1, f2
        ));
    }
    out(String::new());
    out("== scissors (weight=cost*n, kind, fingering) ==".to_string());
    for (_w, c, bg, f1, f2, cnt, pair) in report.scissors.iter().take(15) {
        out(format!(
            "  {}  n={:8.0}  cost={:5.2}  {:3}  {}{}",
            bg, cnt, c, pair, f1, f2
        ));
    }
    out(String::new());
    out("== trigram patterns (chosen fingering, % of trigrams) ==".to_string());
    for k in [
        "alternate",
        "hand_alternation",
        "inroll",
        "outroll",
        "total_rolls",
        "onehand",
        "redirect",
        "redirect_weak",
        "total_redirects",
        "sfb_tri",
        "repeat",
        "other",
    ] {
        out(format!(
            "  {:<16} {:6.2}",
            k,
            pats.get(k).copied().unwrap_or(0.0)
        ));
    }
    out(String::new());
    out("== finger load (% of all strokes) ==".to_string());
    for h in ['L', 'R'] {
        let parts: Vec<String> = ['P', 'R', 'M', 'I']
            .iter()
            .map(|f| {
                format!(
                    "{}={:5.2}%",
                    f,
                    use_.get(&format!("{}{}", h, f)).copied().unwrap_or(0.0)
                )
            })
            .collect();
        out(format!("  {}: {}", h, parts.join("  ")));
    }
    out(String::new());
    out("== alpha fingering guide (trigram DP) ==".to_string());
    for ch in LETTERS.chars() {
        let Some(g) = guide.get(&ch) else { continue };
        let shares: Vec<String> = g
            .share
            .iter()
            .map(|(f, v)| format!("{}={:.1}%", f, v))
            .collect();
        out(format!(
            "  {}@{:<2} default {}  {}",
            ch,
            g.key,
            g.default,
            shares.join(" ")
        ));
        for (f, tops) in &g.alt_contexts {
            let text: Vec<String> = tops
                .iter()
                .take(5)
                .map(|(tg, n)| format!("{}:{:.0}", tg, n))
                .collect();
            out(format!("      alt {}: {}", f, text.join(", ")));
        }
    }
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&out_path, lines.join("\n") + "\n")?;
    if let Some(parent) = out_json.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let guide_json: serde_json::Value = guide
        .iter()
        .map(|(ch, g)| {
            let share: serde_json::Map<String, serde_json::Value> = g
                .share
                .iter()
                .map(|(f, v)| (f.to_string(), json!(v)))
                .collect();
            let alts: serde_json::Map<String, serde_json::Value> = g
                .alt_contexts
                .iter()
                .map(|(f, items)| {
                    (
                        f.to_string(),
                        json!(items
                            .iter()
                            .map(|(tg, n)| json!([tg, n.round_ties_even() as i64]))
                            .collect::<Vec<_>>()),
                    )
                })
                .collect();
            (
                ch.to_string(),
                json!({
                    "key": g.key,
                    "default": g.default.to_string(),
                    "share": share,
                    "alt_contexts": alts,
                }),
            )
        })
        .collect();
    let payload = json!({
        "layout": layout,
        "primary_score": primary_score,
        "primary_corpora": primary_paths,
        "secondary_score": secondary_score,
        "secondary_corpora": secondary_paths,
        "categories": report.cats,
        "worst_bigrams": report.worst.iter().map(|(c,b,f1,f2,n)| json!([c,b,f1.to_string(),f2.to_string(),n])).collect::<Vec<_>>(),
        "scissors": report.scissors.iter().map(|(w,c,b,f1,f2,n,p)| json!([w,c,b,f1.to_string(),f2.to_string(),n,p])).collect::<Vec<_>>(),
        "trigram_patterns": pats,
        "usage": use_,
        "guide": guide_json,
    });
    std::fs::write(&out_json, serde_json::to_string_pretty(&payload)? + "\n")?;
    println!("\nwrote {} and {}", out_path.display(), out_json.display());
    Ok(())
}
