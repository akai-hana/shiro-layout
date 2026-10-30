//! Load, validate, and expose the user-facing YAML configuration.
//!
//! Port of `shirolyzer/utils/config.py`.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use serde_yaml::{Mapping, Value};

use crate::geometry::{finger_index, key_index, FINGERS, KEYS, NK};
use crate::model::{missing_parameters, Scales};
use crate::paths;

pub const PRIORITY_NAMES: [&str; 6] = [
    "sfbs-sfts",
    "scissors",
    "redirects",
    "rolls",
    "alternation",
    "finger_balance",
];

pub const DEFAULT_PRIORITY: &[&[&str]] = &[
    &["sfbs-sfts"],
    &["scissors", "redirects"],
    &["rolls", "alternation"],
    &["finger_balance"],
];

#[derive(Clone, Debug)]
pub struct CorpusEntry {
    pub name: Option<String>,
    pub path: Option<String>,
    pub weight: f64,
}

impl CorpusEntry {
    pub fn label(&self) -> String {
        self.path
            .clone()
            .or_else(|| self.name.clone())
            .unwrap_or_default()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GateMode {
    Only,
    Block,
    Never,
}

#[derive(Clone, Debug)]
pub struct AltGate {
    pub key: char,
    pub finger: char,
    pub mode: GateMode,
    pub context: Option<HashSet<char>>,
    pub explicit: bool,
}

#[derive(Clone, Debug)]
pub struct AltSequence {
    pub sequence: Vec<char>,
    pub fingers: Vec<char>,
}

#[derive(Clone, Debug)]
pub enum AltEntry {
    Gate(AltGate),
    Sequence(AltSequence),
}

pub type FingerSet = HashSet<(usize, usize)>;

#[derive(Clone, Debug)]
pub struct Condition {
    pub kind: String,
    pub allow: bool,
    pub distance: Option<String>,
    pub reverse: bool,
    pub fingers: Option<FingerSet>,
    pub max_extra: f64,
    pub min_width: f64,
    pub deficit: f64,
}

#[derive(Clone, Debug)]
pub struct Config {
    pub path: PathBuf,
    pub raw: Value,
    pub physical_layout: String,
    pub current_layout: String,
    pub primary_corpus: Vec<CorpusEntry>,
    pub secondary_corpus: Vec<CorpusEntry>,
    pub primary_sacrifice: f64,
    pub model_variant: String,
    pub parameters_file: String,
    pub parameters_path: PathBuf,
    pub fingermap: [char; NK],
    pub finger_tokens: Vec<char>,
    pub alt_fingers: Vec<AltEntry>,
    pub conditions: Vec<Condition>,
    pub conditions_are_default: bool,
    pub priority_list: Vec<Vec<String>>,
    pub finger_balance: HashMap<char, f64>,
    pub flat_share: HashMap<char, f64>,
}

impl Config {
    pub fn gate_entries(&self) -> Vec<&AltGate> {
        self.alt_fingers
            .iter()
            .filter_map(|e| match e {
                AltEntry::Gate(g) => Some(g),
                _ => None,
            })
            .collect()
    }

    pub fn sequence_entries(&self) -> Vec<&AltSequence> {
        self.alt_fingers
            .iter()
            .filter_map(|e| match e {
                AltEntry::Sequence(s) => Some(s),
                _ => None,
            })
            .collect()
    }

    pub fn scales(&self) -> Scales {
        let m = self.priority_multipliers();
        Scales {
            sfb: m["sfbs-sfts"],
            scissor: m["scissors"],
            redirect: m["redirects"],
            roll: m["rolls"],
            alt: m["alternation"],
        }
    }

    pub fn balance_weights(&self) -> (f64, f64) {
        let scale = self.priority_multipliers()["finger_balance"];
        (0.4 * scale, 0.5 * scale)
    }

    pub fn priority_multipliers(&self) -> HashMap<String, f64> {
        let mut out: HashMap<String, f64> = PRIORITY_NAMES
            .iter()
            .map(|n| (n.to_string(), 1.0))
            .collect();
        let is_default = self.priority_list.len() == DEFAULT_PRIORITY.len()
            && self
                .priority_list
                .iter()
                .zip(DEFAULT_PRIORITY)
                .all(|(a, b)| a.len() == b.len() && a.iter().zip(*b).all(|(x, y)| x == y));
        if is_default {
            return out;
        }
        let len = self.priority_list.len();
        for (tier, names) in self.priority_list.iter().enumerate() {
            let scale = 1.0 + 0.5 * (len - tier - 1) as f64;
            for name in names {
                out.insert(name.clone(), scale);
            }
        }
        out
    }

    pub fn corpus_path(&self, entry: &CorpusEntry) -> PathBuf {
        let value = entry
            .path
            .clone()
            .or_else(|| entry.name.clone())
            .unwrap_or_default();
        let path = PathBuf::from(shellexpand_home(&value));
        if path.is_file() || path.is_dir() {
            return path;
        }
        let candidate = paths::corpus_root().join(&value).join("parsed");
        if candidate.is_dir() {
            return candidate;
        }
        path
    }
}

fn shellexpand_home(value: &str) -> String {
    if let Some(rest) = value.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return format!("{}/{}", home, rest);
        }
    }
    value.to_string()
}

fn fail(path: &Path, where_: &str, message: &str) -> anyhow::Error {
    anyhow!("{}: {}: {}", path.display(), where_, message)
}

fn as_mapping<'a>(path: &Path, where_: &str, value: &'a Value) -> Result<&'a Mapping> {
    value
        .as_mapping()
        .ok_or_else(|| fail(path, where_, "expected a mapping"))
}

fn get<'a>(map: &'a Mapping, key: &str) -> Option<&'a Value> {
    map.get(Value::String(key.to_string()))
}

fn as_string(path: &Path, where_: &str, value: &Value) -> Result<String> {
    value
        .as_str()
        .map(ToString::to_string)
        .ok_or_else(|| fail(path, where_, "expected a string"))
}

fn as_number(
    path: &Path,
    where_: &str,
    value: &Value,
    minimum: Option<f64>,
    maximum: Option<f64>,
) -> Result<f64> {
    let number = value
        .as_f64()
        .ok_or_else(|| fail(path, where_, "expected a number"))?;
    if let Some(minimum) = minimum {
        if number < minimum {
            return Err(fail(
                path,
                where_,
                &format!("must be >= {minimum}, got {number}"),
            ));
        }
    }
    if let Some(maximum) = maximum {
        if number > maximum {
            return Err(fail(
                path,
                where_,
                &format!("must be <= {maximum}, got {number}"),
            ));
        }
    }
    Ok(number)
}

fn as_bool(path: &Path, where_: &str, value: &Value) -> Result<bool> {
    value
        .as_bool()
        .ok_or_else(|| fail(path, where_, "expected true or false"))
}

fn parse_finger_token(path: &Path, where_: &str, value: &Value) -> Result<char> {
    let text = as_string(path, where_, value)?.to_uppercase();
    let finger = text
        .chars()
        .last()
        .filter(|c| FINGERS.contains(c))
        .ok_or_else(|| {
            fail(
                path,
                where_,
                &format!(
                    "invalid finger token {text:?}; expected P/R/M/I or LP/LR/LM/LI/RP/RR/RM/RI"
                ),
            )
        })?;
    if text.len() > 1 {
        let hand = text.chars().next().unwrap();
        if hand != 'L' && hand != 'R' {
            return Err(fail(path, where_, "invalid finger token"));
        }
    }
    Ok(finger)
}

fn hand_of_key(key: char) -> char {
    if "QWERTASDFGZXCVB".contains(key) {
        'L'
    } else {
        'R'
    }
}

fn parse_hand_finger(path: &Path, where_: &str, key: char, value: &Value) -> Result<char> {
    let text = as_string(path, where_, value)?.to_uppercase();
    let chars: Vec<char> = text.chars().collect();
    if chars.is_empty() {
        return Err(fail(path, where_, "expected a finger"));
    }
    let finger = *chars.last().unwrap();
    if !FINGERS.contains(&finger) {
        return Err(fail(
            path,
            where_,
            &format!("invalid finger {text:?}; expected P/R/M/I or LP/LR/LM/LI/RP/RR/RM/RI"),
        ));
    }
    if chars.len() > 1 {
        let hand = chars[0];
        if hand != 'L' && hand != 'R' {
            return Err(fail(path, where_, "invalid finger token"));
        }
        if hand != hand_of_key(key) {
            return Err(fail(
                path,
                where_,
                &format!("finger {text:?} does not match the hand of key {key:?}"),
            ));
        }
    }
    Ok(finger)
}

fn normalise_corpora(path: &Path, where_: &str, value: Option<&Value>) -> Result<Vec<CorpusEntry>> {
    let value = value.ok_or_else(|| fail(path, where_, "missing corpus list"))?;
    let items: Vec<&Value> = if value.is_sequence() {
        value.as_sequence().unwrap().iter().collect()
    } else if value.is_mapping() || value.is_string() {
        vec![value]
    } else {
        return Err(fail(
            path,
            where_,
            "expected a corpus name/path or a non-empty list of them",
        ));
    };
    if items.is_empty() {
        return Err(fail(path, where_, "expected a non-empty list of corpora"));
    }
    let mut out = Vec::new();
    for (i, item) in items.iter().enumerate() {
        let where_i = format!("{}[{}]", where_, i);
        let (name, path_value, weight_value) = if item.is_string() {
            (Some(item.as_str().unwrap().to_string()), None, None)
        } else if let Some(map) = item.as_mapping() {
            let name = get(map, "name")
                .and_then(Value::as_str)
                .map(ToString::to_string);
            let path_value = get(map, "path")
                .and_then(Value::as_str)
                .map(ToString::to_string);
            (name, path_value, get(map, "weight"))
        } else {
            return Err(fail(
                path,
                &where_i,
                "expected a name/path string or a mapping",
            ));
        };
        if name.is_none() && path_value.is_none() {
            return Err(fail(path, &where_i, "needs a name or path"));
        }
        let weight = match weight_value {
            Some(v) => as_number(path, &format!("{}.weight", where_i), v, Some(0.0), None)?,
            None => 1.0,
        };
        if weight <= 0.0 {
            return Err(fail(
                path,
                &format!("{}.weight", where_i),
                "must be greater than zero",
            ));
        }
        out.push(CorpusEntry {
            name,
            path: path_value,
            weight,
        });
    }
    Ok(out)
}

fn parse_fingermap(path: &Path, value: Option<&Value>) -> Result<([char; NK], Vec<char>)> {
    let value = value.ok_or_else(|| fail(path, "fingermap", "missing fingermap"))?;
    let tokens: Vec<String> = if let Some(seq) = value.as_sequence() {
        seq.iter()
            .map(|v| as_string(path, "fingermap", v))
            .collect::<Result<_>>()?
    } else if let Some(text) = value.as_str() {
        text.split_whitespace().map(ToString::to_string).collect()
    } else {
        return Err(fail(
            path,
            "fingermap",
            "expected a multiline string or a list of tokens",
        ));
    };
    if tokens.len() != NK {
        return Err(fail(
            path,
            "fingermap",
            &format!("expected {} tokens, got {}", NK, tokens.len()),
        ));
    }
    let mut mapping = ['P'; NK];
    let mut order = Vec::with_capacity(NK);
    for (i, token) in tokens.iter().enumerate() {
        let chars: Vec<char> = token.chars().collect();
        if chars.len() != 2 {
            return Err(fail(
                path,
                &format!("fingermap[{}]", i),
                "expected a two-character key/finger token",
            ));
        }
        let key = chars[0];
        let finger = chars[1];
        if key_index(&key.to_string()).is_none() {
            return Err(fail(
                path,
                &format!("fingermap[{}]", i),
                &format!("unknown QWERTY key {key:?}"),
            ));
        }
        if !FINGERS.contains(&finger) {
            return Err(fail(
                path,
                &format!("fingermap[{}]", i),
                &format!("unknown finger {finger:?}; expected one of PRMI"),
            ));
        }
        if key.to_string() != KEYS[i] {
            return Err(fail(
                path,
                &format!("fingermap[{}]", i),
                &format!("expected key {:?} at this position, got {key:?}", KEYS[i]),
            ));
        }
        mapping[i] = finger;
        order.push(finger);
    }
    Ok((mapping, order))
}

fn parse_alt_fingers(path: &Path, value: Option<&Value>) -> Result<Vec<AltEntry>> {
    let value = value.ok_or_else(|| fail(path, "alt_fingers", "missing alt_fingers"))?;
    let seq = value
        .as_sequence()
        .ok_or_else(|| fail(path, "alt_fingers", "expected a list"))?;
    let mut out = Vec::new();
    for (i, item) in seq.iter().enumerate() {
        let where_i = format!("alt_fingers[{}]", i);
        let map = as_mapping(path, &where_i, item)?;
        let has_sequence = get(map, "sequence").is_some();
        let has_key = get(map, "key").is_some();
        if has_sequence == has_key {
            return Err(fail(
                path,
                &where_i,
                "needs exactly one of 'sequence' (explicit override) or 'key' (context gate)",
            ));
        }
        if has_key {
            let key_text = as_string(path, &format!("{}.key", where_i), get(map, "key").unwrap())?;
            let chars: Vec<char> = key_text.to_uppercase().chars().collect();
            if chars.len() != 1 || key_index(&chars[0].to_string()).is_none() {
                return Err(fail(
                    path,
                    &format!("{}.key", where_i),
                    &format!("unknown QWERTY key {key_text:?}"),
                ));
            }
            let key = chars[0];
            let finger = parse_hand_finger(
                path,
                &format!("{}.finger", where_i),
                key,
                get(map, "finger")
                    .ok_or_else(|| fail(path, &where_i, "context gate needs a finger"))?,
            )?;
            let mode_text = as_string(
                path,
                &format!("{}.mode", where_i),
                get(map, "mode").unwrap_or(&Value::String("".into())),
            )?;
            let mode = match mode_text.as_str() {
                "only" => GateMode::Only,
                "block" => GateMode::Block,
                "never" => GateMode::Never,
                _ => {
                    return Err(fail(
                        path,
                        &format!("{}.mode", where_i),
                        "expected only, block, or never",
                    ))
                }
            };
            let mut context = HashSet::new();
            if let Some(ctx) = get(map, "context") {
                let ctx_seq = ctx.as_sequence().ok_or_else(|| {
                    fail(
                        path,
                        &format!("{}.context", where_i),
                        "expected a list of QWERTY keys",
                    )
                })?;
                for (j, c) in ctx_seq.iter().enumerate() {
                    let text = as_string(path, &format!("{}.context[{}]", where_i, j), c)?;
                    let ch = text.to_uppercase().chars().next().unwrap_or('?');
                    if key_index(&ch.to_string()).is_none() {
                        return Err(fail(
                            path,
                            &format!("{}.context[{}]", where_i, j),
                            &format!("unknown QWERTY key {text:?}"),
                        ));
                    }
                    context.insert(ch);
                }
            }
            if mode == GateMode::Never && !context.is_empty() {
                return Err(fail(
                    path,
                    &format!("{}.context", where_i),
                    "mode 'never' does not take a context",
                ));
            }
            let explicit = get(map, "explicit")
                .map(|v| v.as_bool().unwrap_or(false))
                .unwrap_or(false);
            out.push(AltEntry::Gate(AltGate {
                key,
                finger,
                mode,
                context: if context.is_empty() {
                    None
                } else {
                    Some(context)
                },
                explicit,
            }));
        } else {
            let sequence_text = as_string(
                path,
                &format!("{}.sequence", where_i),
                get(map, "sequence").unwrap(),
            )?
            .to_uppercase();
            if sequence_text.is_empty() || sequence_text.chars().count() > 30 {
                return Err(fail(
                    path,
                    &format!("{}.sequence", where_i),
                    "expected 1-30 QWERTY position letters",
                ));
            }
            let sequence: Vec<char> = sequence_text.chars().collect();
            for (j, key) in sequence.iter().enumerate() {
                if key_index(&key.to_string()).is_none() {
                    return Err(fail(
                        path,
                        &format!("{}.sequence[{}]", where_i, j),
                        &format!("unknown QWERTY key {key:?}"),
                    ));
                }
            }
            let fingers = get(map, "fingers")
                .and_then(Value::as_sequence)
                .ok_or_else(|| {
                    fail(
                        path,
                        &format!("{}.fingers", where_i),
                        "expected a finger list",
                    )
                })?;
            if fingers.len() != sequence.len() {
                return Err(fail(
                    path,
                    &format!("{}.fingers", where_i),
                    "expected one finger token per sequence position",
                ));
            }
            let mut parsed = Vec::new();
            for (j, (key, finger)) in sequence.iter().zip(fingers.iter()).enumerate() {
                parse_hand_finger(path, &format!("{}.fingers[{}]", where_i, j), *key, finger)?;
                parsed.push(parse_finger_token(path, &where_i, finger)?);
            }
            out.push(AltEntry::Sequence(AltSequence {
                sequence,
                fingers: parsed,
            }));
        }
    }
    Ok(out)
}

fn finger_set(path: &Path, where_: &str, value: Option<&Value>) -> Result<Option<FingerSet>> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    let seq = value
        .as_sequence()
        .ok_or_else(|| fail(path, where_, "expected a list of hand/finger tokens"))?;
    if seq.is_empty() {
        return Ok(None);
    }
    let mut out = HashSet::new();
    for (i, token) in seq.iter().enumerate() {
        let text = as_string(path, &format!("{}[{}]", where_, i), token)?.to_uppercase();
        let chars: Vec<char> = text.chars().collect();
        let finger = *chars
            .last()
            .ok_or_else(|| fail(path, where_, "invalid finger token"))?;
        let fi = finger_index(finger).ok_or_else(|| fail(path, where_, "invalid finger token"))?;
        if chars.len() == 1 {
            out.insert((0, fi));
            out.insert((1, fi));
        } else {
            let hand = if chars[0] == 'L' { 0 } else { 1 };
            out.insert((hand, fi));
        }
    }
    Ok(Some(out))
}

fn parse_conditions(path: &Path, value: Option<&Value>) -> Result<Vec<Condition>> {
    let value = value.ok_or_else(|| fail(path, "conditions", "missing conditions"))?;
    let seq = value
        .as_sequence()
        .ok_or_else(|| fail(path, "conditions", "expected a non-empty list"))?;
    if seq.is_empty() {
        return Err(fail(path, "conditions", "expected a non-empty list"));
    }
    let mut out = Vec::new();
    for (i, item) in seq.iter().enumerate() {
        let where_i = format!("conditions[{}]", i);
        let map = as_mapping(path, &where_i, item)?;
        let kind = as_string(
            path,
            &format!("{}.type", where_i),
            get(map, "type").unwrap_or(&Value::String("".into())),
        )?;
        if !["scissors", "spread", "separation", "crossing"].contains(&kind.as_str()) {
            return Err(fail(
                path,
                &format!("{}.type", where_i),
                "expected scissors, spread, separation, or crossing",
            ));
        }
        let allow = as_bool(
            path,
            &format!("{}.allow", where_i),
            get(map, "allow").ok_or_else(|| fail(path, &where_i, "missing allow"))?,
        )?;
        let mut distance = None;
        let mut reverse = false;
        let mut fingers = None;
        let mut max_extra = 2.0;
        let mut min_width = 1.9;
        let mut deficit = 0.30;
        if kind == "scissors" {
            let dist = as_string(
                path,
                &format!("{}.distance", where_i),
                get(map, "distance").unwrap_or(&Value::String("".into())),
            )?;
            if dist != "full" && dist != "half" {
                return Err(fail(
                    path,
                    &format!("{}.distance", where_i),
                    "expected full or half",
                ));
            }
            distance = Some(dist);
            if let Some(v) = get(map, "reverse") {
                reverse = as_bool(path, &format!("{}.reverse", where_i), v)?;
            }
            fingers = finger_set(path, &format!("{}.fingers", where_i), get(map, "fingers"))?;
        } else if kind == "spread" {
            if let Some(v) = get(map, "max_extra") {
                max_extra = as_number(path, &format!("{}.max_extra", where_i), v, Some(0.0), None)?;
            }
        } else if kind == "separation" {
            if let Some(v) = get(map, "min_width") {
                min_width = as_number(path, &format!("{}.min_width", where_i), v, Some(0.0), None)?;
            }
            fingers = finger_set(path, &format!("{}.fingers", where_i), get(map, "fingers"))?;
        } else if let Some(v) = get(map, "deficit") {
            deficit = as_number(path, &format!("{}.deficit", where_i), v, Some(0.0), None)?;
        }
        out.push(Condition {
            kind,
            allow,
            distance,
            reverse,
            fingers,
            max_extra,
            min_width,
            deficit,
        });
    }
    Ok(out)
}

fn parse_priority(path: &Path, value: Option<&Value>) -> Result<Vec<Vec<String>>> {
    let value = value.ok_or_else(|| fail(path, "priority_list", "missing priority_list"))?;
    let seq = value
        .as_sequence()
        .ok_or_else(|| fail(path, "priority_list", "expected a non-empty list"))?;
    if seq.is_empty() {
        return Err(fail(path, "priority_list", "expected a non-empty list"));
    }
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for (i, item) in seq.iter().enumerate() {
        let names: Vec<String> = if item.is_string() {
            vec![item.as_str().unwrap().to_string()]
        } else if let Some(items) = item.as_sequence() {
            items
                .iter()
                .map(|v| as_string(path, &format!("priority_list[{}]", i), v))
                .collect::<Result<_>>()?
        } else {
            return Err(fail(
                path,
                &format!("priority_list[{}]", i),
                "expected a name or a non-empty list of names",
            ));
        };
        if names.is_empty() {
            return Err(fail(
                path,
                &format!("priority_list[{}]", i),
                "expected a name or a non-empty list of names",
            ));
        }
        let mut tier = Vec::new();
        for (j, name) in names.iter().enumerate() {
            if !PRIORITY_NAMES.contains(&name.as_str()) {
                return Err(fail(
                    path,
                    &format!("priority_list[{}][{}]", i, j),
                    &format!("unknown priority {name:?}; expected one of {PRIORITY_NAMES:?}"),
                ));
            }
            if !seen.insert(name.clone()) {
                return Err(fail(
                    path,
                    &format!("priority_list[{}][{}]", i, j),
                    &format!("duplicate priority {name:?}"),
                ));
            }
            tier.push(name.clone());
        }
        out.push(tier);
    }
    for name in PRIORITY_NAMES {
        if !seen.contains(name) {
            return Err(fail(
                path,
                "priority_list",
                &format!("missing priority {name:?}"),
            ));
        }
    }
    Ok(out)
}

fn parse_balance(
    path: &Path,
    value: Option<&Value>,
) -> Result<(HashMap<char, f64>, HashMap<char, f64>)> {
    let value = value.ok_or_else(|| fail(path, "finger_balance", "missing finger_balance"))?;
    let map = as_mapping(path, "finger_balance", value)?;
    let mut vals: HashMap<String, f64> = HashMap::new();
    for (k, v) in map {
        let key = as_string(path, "finger_balance key", k)?;
        let ok = key.len() <= 2
            && key
                .chars()
                .last()
                .map(|c| FINGERS.contains(&c))
                .unwrap_or(false)
            && (key.len() == 1 || key.starts_with('L') || key.starts_with('R'));
        if !ok {
            return Err(fail(
                path,
                &format!("finger_balance.{}", key),
                "expected LP/LR/LM/LI/RP/RR/RM/RI or P/R/M/I",
            ));
        }
        let number = as_number(path, &format!("finger_balance.{}", key), v, Some(0.0), None)?;
        vals.insert(key, number);
    }
    let mut per_finger = HashMap::new();
    for finger in FINGERS {
        let mut readings = Vec::new();
        if let Some(v) = vals.get(&finger.to_string()) {
            readings.push(*v);
        }
        for hand in ['L', 'R'] {
            if let Some(v) = vals.get(&format!("{}{}", hand, finger)) {
                readings.push(*v);
            }
        }
        if readings.is_empty() {
            return Err(fail(
                path,
                &format!("finger_balance.{}", finger),
                "missing percentage for this finger",
            ));
        }
        per_finger.insert(finger, readings.iter().sum::<f64>() / readings.len() as f64);
    }
    let total: f64 = per_finger.values().sum();
    if total <= 0.0 {
        return Err(fail(
            path,
            "finger_balance",
            "at least one percentage must be positive",
        ));
    }
    let flat = per_finger.iter().map(|(k, v)| (*k, *v / total)).collect();
    Ok((per_finger, flat))
}

fn default_raw() -> Result<Value> {
    let text = paths::read_to_string(&paths::config_file())?;
    serde_yaml::from_str(&text).context("default config")
}

pub fn load(path: Option<&Path>) -> Result<Config> {
    let path = path
        .map(Path::to_path_buf)
        .unwrap_or_else(paths::config_file);
    let text = paths::read_to_string(&path)?;
    let raw: Value =
        serde_yaml::from_str(&text).with_context(|| format!("{}: invalid YAML", path.display()))?;
    let root = as_mapping(&path, "root", &raw)?;
    let known = [
        "general",
        "model",
        "fingermap",
        "alt_fingers",
        "conditions",
        "priority_list",
        "finger_balance",
    ];
    for key in root.keys() {
        let name = key.as_str().unwrap_or("?");
        if !known.contains(&name) {
            return Err(fail(&path, "root", &format!("unknown section {name:?}")));
        }
    }
    for section in known {
        if get(root, section).is_none() {
            return Err(fail(
                &path,
                "root",
                &format!("missing required section {section:?}"),
            ));
        }
    }

    let general = as_mapping(&path, "general", get(root, "general").unwrap())?;
    let physical_layout = as_string(
        &path,
        "general.physical_layout",
        get(general, "physical_layout").unwrap_or(&Value::String("".into())),
    )?;
    if physical_layout != "ansi" {
        return Err(fail(
            &path,
            "general.physical_layout",
            &format!("unknown physical layout {physical_layout:?}; expected ansi"),
        ));
    }
    let current_layout = as_string(
        &path,
        "general.current_layout",
        get(general, "current_layout").ok_or_else(|| {
            fail(
                &path,
                "general.current_layout",
                "missing required 30-character layout",
            )
        })?,
    )?;
    if current_layout.chars().count() != 30 {
        return Err(fail(
            &path,
            "general.current_layout",
            "expected a 30-character layout",
        ));
    }
    let primary_corpus = normalise_corpora(
        &path,
        "general.primary_corpus",
        get(general, "primary_corpus"),
    )?;
    let secondary_corpus = normalise_corpora(
        &path,
        "general.secondary_corpus",
        get(general, "secondary_corpus"),
    )?;
    let primary_sacrifice = match get(general, "primary_sacrifice") {
        Some(v) => as_number(&path, "general.primary_sacrifice", v, Some(0.0), None)?,
        None => 0.00023,
    };

    let model = as_mapping(&path, "model", get(root, "model").unwrap())?;
    let model_type = as_string(
        &path,
        "model.type",
        get(model, "type").unwrap_or(&Value::String("".into())),
    )?;
    if model_type != "pattern" {
        return Err(fail(
            &path,
            "model.type",
            "only the 'pattern' model is implemented",
        ));
    }
    let variant = as_string(
        &path,
        "model.variant",
        get(model, "variant").unwrap_or(&Value::String("A".into())),
    )?;
    if variant != "A" && variant != "B" {
        return Err(fail(&path, "model.variant", "expected A or B"));
    }
    let parameters_file = as_string(
        &path,
        "model.parameters_file",
        get(model, "parameters_file").unwrap_or(&Value::String("".into())),
    )?;
    let parameters_path = {
        let p = PathBuf::from(&parameters_file);
        if p.is_absolute() {
            p
        } else {
            paths::config_dir().join(p)
        }
    };
    if !parameters_path.is_file() {
        return Err(fail(
            &path,
            "model.parameters_file",
            &format!("parameter file not found: {}", parameters_path.display()),
        ));
    }
    let params_json = serde_json::from_str(&paths::read_to_string(&parameters_path)?)?;
    let missing = missing_parameters(&params_json);
    if !missing.is_empty() {
        return Err(fail(
            &path,
            "model.parameters_file",
            &format!(
                "{} is missing parameters: {:?}",
                parameters_path.display(),
                missing
            ),
        ));
    }

    let (fingermap, finger_tokens) = parse_fingermap(&path, get(root, "fingermap"))?;
    let alt_fingers = parse_alt_fingers(&path, get(root, "alt_fingers"))?;
    let conditions = parse_conditions(&path, get(root, "conditions"))?;
    let priority_list = parse_priority(&path, get(root, "priority_list"))?;
    let (finger_balance, flat_share) = parse_balance(&path, get(root, "finger_balance"))?;
    let conditions_are_default = match default_raw() {
        Ok(default) => get(default.as_mapping().unwrap(), "conditions") == get(root, "conditions"),
        Err(_) => false,
    };

    Ok(Config {
        path,
        raw,
        physical_layout,
        current_layout,
        primary_corpus,
        secondary_corpus,
        primary_sacrifice,
        model_variant: variant,
        parameters_file,
        parameters_path,
        fingermap,
        finger_tokens,
        alt_fingers,
        conditions,
        conditions_are_default,
        priority_list,
        finger_balance,
        flat_share,
    })
}

/// Build the model from a loaded config.
pub fn build_model(cfg: &Config) -> Result<crate::model::Model> {
    let mut model = crate::model::Model::from_file(
        &cfg.parameters_path,
        cfg.model_variant == "A",
        cfg.fingermap,
    )?;
    model.scales = cfg.scales();
    Ok(model)
}
