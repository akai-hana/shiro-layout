use anyhow::Result;

use shirolyzer::context;
use shirolyzer::geometry::{finger_index, key_index, FINGERS, KEYS, NF};
use shirolyzer::model::pt_idx;
use shirolyzer::scorer::{big_idx, tri_idx, PRESS};

fn letter_id(ch: char) -> Option<usize> {
    match ch {
        'a'..='z' => Some(ch as usize - 'a' as usize),
        ' ' => Some(shirolyzer::corpus::SPACE_INDEX),
        _ => None,
    }
}

fn keymap(layout: &str) -> [Option<usize>; 27] {
    let mut map = [None; 27];
    for (i, ch) in layout.chars().enumerate() {
        if let (Some(id), Some(key)) = (letter_id(ch), key_index(KEYS[i])) {
            map[id] = Some(key);
        }
    }
    map
}

fn key_char(k: usize) -> char {
    KEYS[k].chars().next().unwrap()
}

fn hand_finger(hand: usize, f: usize) -> String {
    let h = if hand == 0 { 'L' } else { 'R' };
    format!("{h}{}", FINGERS[f])
}

fn parse_desired(
    tokens: &[&str],
    keys: &[usize],
    geom: &shirolyzer::geometry::Geometry,
) -> Result<Vec<usize>, String> {
    let mut out = Vec::new();
    for (pos, token) in tokens.iter().enumerate() {
        let text = token.to_uppercase();
        let chars: Vec<char> = text.chars().collect();
        let f = *chars.last().unwrap();
        let fi = finger_index(f).ok_or_else(|| format!("bad finger {token:?}"))?;
        if chars.len() > 1 {
            let hand = chars[0];
            let expected = if geom.hand[keys[pos]] == 0 { 'L' } else { 'R' };
            if hand != expected {
                return Err(format!(
                    "{token} on {} is wrong hand (needs {expected})",
                    key_char(keys[pos])
                ));
            }
        }
        out.push(fi);
    }
    Ok(out)
}

fn tri_cost(ctx: &context::Context, keys: &[usize], fs: &[usize]) -> (f64, f64, f64) {
    let chars: Vec<char> = keys.iter().map(|&k| key_char(k)).collect();
    let mut pm = vec![0.0; NF * NF * NF];
    ctx.gates
        .penalty_mask_into(&chars, &ctx.conditions, &mut pm);
    let mut cond = vec![0.0; NF * NF * NF];
    ctx.conditions.penalty_mask_into(&chars, &mut cond);
    let mut costs = vec![0.0; NF * NF * NF];
    ctx.model.tri_many_into(
        &ctx.geom,
        &ctx.tables.model,
        &[keys[0]],
        &[keys[1]],
        &[keys[2]],
        &mut costs,
    );
    let idx = (fs[0] * NF + fs[1]) * NF + fs[2];
    (costs[idx] + 3.0 * PRESS, pm[idx], cond[idx])
}

fn bi_cost(ctx: &context::Context, keys: &[usize], fs: &[usize]) -> (f64, f64, f64) {
    let chars: Vec<char> = keys.iter().map(|&k| key_char(k)).collect();
    let mut pm = vec![0.0; NF * NF];
    ctx.gates
        .penalty_mask_into(&chars, &ctx.conditions, &mut pm);
    let mut cond = vec![0.0; NF * NF];
    ctx.conditions.penalty_mask_into(&chars, &mut cond);
    if keys[0] == keys[1] && fs[0] != fs[1] {
        return (f64::INFINITY, 0.0, 0.0);
    }
    (
        ctx.tables.model.t2[pt_idx(keys[0], fs[0], keys[1], fs[1])] + 2.0 * PRESS,
        pm[fs[0] * NF + fs[1]],
        cond[fs[0] * NF + fs[1]],
    )
}

struct Entry {
    ngram: &'static str,
    desired: &'static [&'static [&'static str]],
}

fn main() -> Result<()> {
    let layout = "ztiuqvgrnhekyaocslpd;j,./wxbfm";
    let ctx = context::Context::load(None)?;
    let m = keymap(layout);

    let entries = [
        Entry {
            ngram: "oje",
            desired: &[&["I", "R", "P"]],
        },
        Entry {
            ngram: "ayi",
            desired: &[&["I", "M", "R"]],
        },
        Entry {
            ngram: "xch",
            desired: &[&["RI", "LI", "RR"]],
        },
        Entry {
            ngram: "oyi",
            desired: &[&["I", "M", "R"]],
        },
        Entry {
            ngram: "osc",
            desired: &[&["LI", "RI", "LI"]],
        },
        Entry {
            ngram: "itz",
            desired: &[&["I", "M", "R"]],
        },
        Entry {
            ngram: "rph",
            desired: &[&["I", "M", "R"]],
        },
        Entry {
            ngram: "npl",
            desired: &[&["M", "I", "I"]],
        },
        Entry {
            ngram: "rsc",
            desired: &[&["RM", "RI", "LI"]],
        },
        Entry {
            ngram: "aje",
            desired: &[&["I", "R", "P"]],
        },
        Entry {
            ngram: "kti",
            desired: &[&["I", "M", "I"]],
        },
        Entry {
            ngram: "okt",
            desired: &[&["I", "M", "R"]],
        },
        Entry {
            ngram: "xv",
            desired: &[&["RI", "LI"]],
        },
        Entry {
            ngram: "fd",
            desired: &[&["I", "R"]],
        },
        Entry {
            ngram: "wv",
            desired: &[&["RI", "LI"]],
        },
        Entry {
            ngram: "xg",
            desired: &[&["I", "M"]],
        },
        Entry {
            ngram: "xs",
            desired: &[&["I", "M"]],
        },
        Entry {
            ngram: "hf",
            desired: &[&["M", "I"]],
        },
        Entry {
            ngram: "wc",
            desired: &[&["RI", "LI"], &["LI", "LM"]],
        },
        Entry {
            ngram: "cw",
            desired: &[&["LI", "RI"], &["LM", "LI"]],
        },
        Entry {
            ngram: "pn",
            desired: &[&["I", "M"]],
        },
        Entry {
            ngram: "nb",
            desired: &[&["M", "I"]],
        },
        Entry {
            ngram: "vx",
            desired: &[&["LI", "RI"]],
        },
        Entry {
            ngram: "df",
            desired: &[&["R", "I"]],
        },
        Entry {
            ngram: "vw",
            desired: &[&["LI", "RI"]],
        },
        Entry {
            ngram: "gx",
            desired: &[&["M", "I"]],
        },
        Entry {
            ngram: "sx",
            desired: &[&["M", "I"]],
        },
        Entry {
            ngram: "fh",
            desired: &[&["I", "M"]],
        },
        Entry {
            ngram: "np",
            desired: &[&["M", "I"]],
        },
        Entry {
            ngram: "bn",
            desired: &[&["I", "M"]],
        },
        Entry {
            ngram: "ke",
            desired: &[&["R", "P"]],
        },
        Entry {
            ngram: "ld",
            desired: &[&["M", "P"]],
        },
        Entry {
            ngram: "ey",
            desired: &[&["P", "M"]],
        },
        Entry {
            ngram: "ye",
            desired: &[&["M", "P"]],
        },
        Entry {
            ngram: "ph",
            desired: &[&["M", "R"], &["I", "M"]],
        },
        Entry {
            ngram: "mb",
            desired: &[&["R", "I"]],
        },
        Entry {
            ngram: "dl",
            desired: &[&["P", "M"]],
        },
        Entry {
            ngram: "ek",
            desired: &[&["P", "R"]],
        },
        Entry {
            ngram: "mf",
            desired: &[&["P", "R"]],
        },
        Entry {
            ngram: "pd",
            desired: &[&["R", "P"]],
        },
        Entry {
            ngram: "bm",
            desired: &[&["M", "P"]],
        },
        Entry {
            ngram: "hp",
            desired: &[&["R", "M"], &["M", "I"]],
        },
        Entry {
            ngram: "dp",
            desired: &[&["P", "R"]],
        },
        Entry {
            ngram: "fm",
            desired: &[&["R", "P"]],
        },
    ];

    for e in entries {
        let ids: Vec<usize> = e
            .ngram
            .chars()
            .map(|ch| m[letter_id(ch).unwrap()].unwrap())
            .collect();
        let keys: Vec<String> = ids.iter().map(|&k| key_char(k).to_string()).collect();
        let n = ids.len();
        let (cur_fs, cur_cost) = if n == 3 {
            let idx = tri_idx(ids[0], ids[1], ids[2]);
            let enc = ctx.tables.tri_f[idx];
            let fs = [enc[0] as usize, enc[1] as usize, enc[2] as usize];
            (fs.to_vec(), ctx.tables.tri[idx])
        } else {
            let idx = big_idx(ids[0], ids[1]);
            let enc = ctx.tables.big_f[idx] as usize;
            let fs = [enc / 5, enc % 5];
            (fs.to_vec(), ctx.tables.big[idx])
        };
        let cur_tokens: Vec<String> = cur_fs
            .iter()
            .enumerate()
            .map(|(pos, &f)| hand_finger(ctx.geom.hand[ids[pos]], f))
            .collect();
        println!(
            "\n{} -> {}  current {} cost={:.3}",
            e.ngram,
            keys.join(" "),
            cur_tokens.join("-"),
            cur_cost
        );
        for alt in e.desired {
            match parse_desired(alt, &ids, &ctx.geom) {
                Err(msg) => println!("  desired {:<14} INVALID: {msg}", alt.join("-")),
                Ok(fs) => {
                    let (raw, pen, cond) = if n == 3 {
                        tri_cost(&ctx, &ids, &fs)
                    } else {
                        bi_cost(&ctx, &ids, &fs)
                    };
                    let cost = raw + pen;
                    let tokens: Vec<String> = fs
                        .iter()
                        .enumerate()
                        .map(|(pos, &f)| hand_finger(ctx.geom.hand[ids[pos]], f))
                        .collect();
                    let delta = cost - cur_cost;
                    println!(
                        "  desired {:<14} {} cost={:.3} raw={:.3} gate={:.3} cond={:.3} delta={:+.3}",
                        alt.join("-"),
                        tokens.join("-"),
                        cost,
                        raw,
                        pen,
                        cond,
                        delta
                    );
                    if raw.is_infinite() {
                        for (pos, &f) in fs.iter().enumerate() {
                            if !ctx.geom.reach[ids[pos]][f] {
                                println!(
                                    "    unreachable: {} by {}",
                                    key_char(ids[pos]),
                                    hand_finger(ctx.geom.hand[ids[pos]], f)
                                );
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
