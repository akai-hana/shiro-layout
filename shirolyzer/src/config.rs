//! Load, validate, and expose the user-facing YAML configuration.
//!
//! Port of `shirolyzer/utils/config.py`.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use serde_yaml::{Mapping, Value};

use crate::geometry::{finger_index, key_index, FINGERS, KEYS, NF, NFINGER, NK};
use crate::model::{missing_parameters, Scales};
use crate::paths;
use crate::redirect::{is_turn, RedirectTiers};
use crate::roll::{is_adjacent, RollTiers};
use crate::scissor::{ScissorTiers, SCISSOR_BANDS};

pub const PRIORITY_NAMES: [&str; 6] = [
    "sfbs-sfts",
    "scissors",
    "redirects",
    "rolls",
    "alternation",
    "finger_balance",
];

/// Weight of every priority term for a tier list.  With N tiers, tier t
/// (0 = highest) is multiplied by `1 + 0.5 * (N - t - 1)`.  The list is always
/// applied as written: there is no special "default" ordering.  To weight all
/// terms equally, put them in a single tier.  Terms missing from the list are
/// switched off (multiplier 0.0).
pub fn priority_weights(priority_list: &[Vec<String>]) -> HashMap<String, f64> {
    let mut out: HashMap<String, f64> = PRIORITY_NAMES
        .iter()
        .map(|n| (n.to_string(), 0.0))
        .collect();
    let len = priority_list.len();
    for (tier, names) in priority_list.iter().enumerate() {
        let scale = 1.0 + 0.5 * (len - tier - 1) as f64;
        for name in names {
            out.insert(name.clone(), scale);
        }
    }
    out
}

/// Priority terms that a tier list leaves out (and therefore disables).
pub fn omitted_priorities(priority_list: &[Vec<String>]) -> Vec<&'static str> {
    PRIORITY_NAMES
        .iter()
        .copied()
        .filter(|name| !priority_list.iter().flatten().any(|n| n == name))
        .collect()
}

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
pub enum AltMode {
    Allow,
    Permit,
    Block,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BalanceSource {
    Default,
    #[default]
    Chosen,
}

impl BalanceSource {
    pub fn label(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Chosen => "chosen",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FingeringMode {
    Fixed,
    #[default]
    Flexible,
}

impl FingeringMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Fixed => "fixed",
            Self::Flexible => "flexible",
        }
    }
}

#[derive(Clone, Debug)]
pub struct AltRule {
    pub keys: Vec<char>,
    pub fingers: Vec<usize>,
    pub mode: AltMode,
}

#[derive(Clone, Debug, Default)]
pub struct Sacrifice {
    pub score: Option<f64>,
    pub sfb_sft: Option<f64>,
    pub scissors: Option<f64>,
    pub redirects: Option<f64>,
    pub rolls: Option<f64>,
    pub alternation: Option<f64>,
    pub finger_balance: Option<f64>,
}

impl Sacrifice {
    pub fn score_only(percent: f64) -> Self {
        Self {
            score: Some(percent),
            ..Self::default()
        }
    }

    pub fn has_metrics(&self) -> bool {
        self.sfb_sft.is_some()
            || self.scissors.is_some()
            || self.redirects.is_some()
            || self.rolls.is_some()
            || self.alternation.is_some()
            || self.finger_balance.is_some()
    }

    pub fn all_metrics(&self) -> bool {
        self.sfb_sft.is_some()
            && self.scissors.is_some()
            && self.redirects.is_some()
            && self.rolls.is_some()
            && self.alternation.is_some()
            && self.finger_balance.is_some()
    }
}

pub type FingerSet = HashSet<(usize, usize)>;
pub type FingerPair = [(usize, usize); 2];

#[derive(Clone, Debug)]
pub struct Condition {
    pub kind: String,
    pub allow: bool,
    pub distance: Option<String>,
    pub reverse: bool,
    pub fingers: Option<FingerSet>,
    pub pairs: Option<Vec<FingerPair>>,
    pub max_extra: f64,
    pub min_width: f64,
    pub deficit: f64,
}

#[derive(Clone, Debug)]
pub struct Config {
    pub path: PathBuf,
    pub raw: Value,
    pub geometry: PathBuf,
    pub punctuation: Vec<char>,
    pub primary_corpus: Vec<CorpusEntry>,
    pub secondary_corpus: Vec<CorpusEntry>,
    pub primary_sacrifice: Sacrifice,
    pub disallow_reverse_scissors: [[bool; NFINGER]; NFINGER],
    pub parameters_file: String,
    pub parameters_path: PathBuf,
    pub fingering: FingeringMode,
    pub fingermap: [usize; NK],
    pub finger_tokens: Vec<char>,
    pub alt_fingers: Vec<AltRule>,
    pub conditions: Vec<Condition>,
    pub conditions_are_default: bool,
    pub priority_list: Vec<Vec<String>>,
    pub redirect_tiers: Option<RedirectTiers>,
    /// Extra multiplier on every redirect cost, applied on top of the
    /// `redirects` priority tier (1.0 = no change).
    pub redirect_scale: f64,
    pub roll_tiers: Option<RollTiers>,
    pub scissor_tiers: Option<ScissorTiers>,
    pub finger_balance: HashMap<char, f64>,
    pub flat_share: HashMap<char, f64>,
    pub finger_strength: [f64; NF],
    pub balance_source: BalanceSource,
    pub hand_restrictions: [Option<usize>; 26],
}

impl Config {
    pub fn scales(&self) -> Scales {
        let m = self.priority_multipliers();
        Scales {
            sfb: m["sfbs-sfts"],
            scissor: m["scissors"],
            redirect: m["redirects"] * self.redirect_scale,
            roll: m["rolls"],
            alt: m["alternation"],
            redirect_costs: self.redirect_tiers.as_ref().map(RedirectTiers::cost_table),
            roll_credits: self.roll_tiers.as_ref().map(RollTiers::credit_table),
            scissor_tiers: self.scissor_tiers.as_ref().map(ScissorTiers::table),
        }
    }

    pub fn balance_weights(&self) -> (f64, f64) {
        let scale = self.priority_multipliers()["finger_balance"];
        (0.4 * scale, 0.5 * scale)
    }

    pub fn priority_multipliers(&self) -> HashMap<String, f64> {
        priority_weights(&self.priority_list)
    }

    /// `sfbs-sfts ×3.0 · scissors ×2.5 · ...` in tier order, for display.
    pub fn priority_summary(&self) -> String {
        let weights = self.priority_multipliers();
        self.priority_list
            .iter()
            .flatten()
            .map(|name| {
                if name == "redirects" && self.redirect_scale != 1.0 {
                    format!(
                        "{name} ×{:.1} (tier ×{:.1} · redirect_scale ×{})",
                        weights[name] * self.redirect_scale,
                        weights[name],
                        self.redirect_scale
                    )
                } else {
                    format!("{name} ×{:.1}", weights[name])
                }
            })
            .collect::<Vec<_>>()
            .join(" · ")
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

fn hand_of_key(key: char) -> char {
    if "QWERTASDFGZXCVB".contains(key) {
        'L'
    } else {
        'R'
    }
}

fn hand_of_key_index(key: char) -> usize {
    if hand_of_key(key) == 'L' {
        0
    } else {
        1
    }
}

/// Parse a finger token to an eight-state index.  A bare `P/R/M/I` uses
/// `default_hand`; an explicit `LP/LR/LM/LI/RP/RR/RM/RI` always wins, which is
/// how cross-hand alt rules are requested.
fn parse_finger_state(
    path: &Path,
    where_: &str,
    value: &Value,
    default_hand: Option<usize>,
) -> Result<usize> {
    let text = as_string(path, where_, value)?.to_uppercase();
    let chars: Vec<char> = text.chars().collect();
    if chars.is_empty() || chars.len() > 2 {
        return Err(fail(
            path,
            where_,
            &format!("invalid finger token {text:?}; expected P/R/M/I or LP/LR/LM/LI/RP/RR/RM/RI"),
        ));
    }
    let finger = *chars.last().unwrap();
    let kind = finger_index(finger).ok_or_else(|| {
        fail(
            path,
            where_,
            &format!("invalid finger token {text:?}; expected P/R/M/I or LP/LR/LM/LI/RP/RR/RM/RI"),
        )
    })?;
    let hand = if chars.len() == 1 {
        default_hand.ok_or_else(|| {
            fail(
                path,
                where_,
                &format!("finger {text:?} needs an explicit L/R hand in this context"),
            )
        })?
    } else {
        match chars[0] {
            'L' => 0,
            'R' => 1,
            _ => return Err(fail(path, where_, "invalid finger token")),
        }
    };
    Ok(hand * NFINGER + kind)
}

fn parse_percent(path: &Path, where_: &str, value: &Value) -> Result<f64> {
    let number = if let Some(text) = value.as_str() {
        let text = text.trim();
        let raw = text.strip_suffix('%').ok_or_else(|| {
            fail(
                path,
                where_,
                &format!("expected a percentage ending in %, got {text:?}"),
            )
        })?;
        raw.trim()
            .parse::<f64>()
            .map_err(|_| fail(path, where_, &format!("invalid percentage {text:?}")))?
    } else {
        as_number(path, where_, value, Some(0.0), None)?
    };
    if !number.is_finite() || number < 0.0 {
        return Err(fail(path, where_, "expected a non-negative percentage"));
    }
    Ok(number)
}

fn parse_fingering(path: &Path, value: Option<&Value>) -> Result<FingeringMode> {
    let Some(value) = value else {
        return Ok(FingeringMode::Flexible);
    };
    let text = as_string(path, "general.fingering", value)?;
    match text.trim().to_ascii_lowercase().as_str() {
        "fixed" | "default" => Ok(FingeringMode::Fixed),
        "flexible" | "free" => Ok(FingeringMode::Flexible),
        other => Err(fail(
            path,
            "general.fingering",
            &format!("unknown mode {other:?}; expected \"fixed\" or \"flexible\""),
        )),
    }
}

fn parse_sacrifice(path: &Path, value: Option<&Value>) -> Result<Sacrifice> {
    let Some(value) = value else {
        return Ok(Sacrifice::score_only(0.00023));
    };
    if !value.is_mapping() {
        return Ok(Sacrifice::score_only(as_number(
            path,
            "general.primary_sacrifice",
            value,
            Some(0.0),
            None,
        )?));
    }
    let map = as_mapping(path, "general.primary_sacrifice", value)?;
    let mut out = Sacrifice::default();
    for (key, item) in map {
        let key = key.as_str().unwrap_or_default();
        let where_ = format!("general.primary_sacrifice.{key}");
        let parsed = parse_percent(path, &where_, item)?;
        match key {
            "score" => out.score = Some(parsed),
            "sfb" | "sfbs" | "sft" | "sfts" | "sfb_sft" => out.sfb_sft = Some(parsed),
            "scissors" => out.scissors = Some(parsed),
            "redirects" => out.redirects = Some(parsed),
            "rolls" => out.rolls = Some(parsed),
            "alternation" => out.alternation = Some(parsed),
            "finger_balance" | "balance" => out.finger_balance = Some(parsed),
            _ => {
                return Err(fail(
                    path,
                    "general.primary_sacrifice",
                    &format!(
                        "unknown metric {key:?}; expected score, sfb/sfts, scissors, redirects, rolls, alternation, or finger_balance"
                    ),
                ))
            }
        }
    }
    if out.score.is_none() && !out.all_metrics() {
        return Err(fail(
            path,
            "general.primary_sacrifice",
            "omit score only when every metric (sfbs/sfts, scissors, redirects, rolls, alternation, finger_balance) is present",
        ));
    }
    Ok(out)
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

fn parse_fingermap(path: &Path, value: Option<&Value>) -> Result<([usize; NK], Vec<char>)> {
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
    let mut mapping = [0usize; NK];
    let mut order = Vec::with_capacity(NK);
    for (i, token) in tokens.iter().enumerate() {
        let chars: Vec<char> = token.to_uppercase().chars().collect();
        if chars.len() != 2 || !matches!(chars[0], 'L' | 'R') || !FINGERS.contains(&chars[1]) {
            return Err(fail(
                path,
                &format!("fingermap[{}]", i),
                &format!("expected a hand/finger token (LP/LR/LM/LI/RI/RM/RR/RP), got {token:?}"),
            ));
        }
        let hand = chars[0];
        let finger = chars[1];
        let expected_hand = if hand_of_key(KEYS[i].chars().next().unwrap()) == 'L' {
            'L'
        } else {
            'R'
        };
        if hand != expected_hand {
            return Err(fail(
                path,
                &format!("fingermap[{}]", i),
                &format!(
                    "finger {token:?} does not match the {:?} hand of key {:?}",
                    if expected_hand == 'L' {
                        "left"
                    } else {
                        "right"
                    },
                    KEYS[i]
                ),
            ));
        }
        mapping[i] = if hand == 'L' {
            finger_index(finger).unwrap()
        } else {
            NFINGER + finger_index(finger).unwrap()
        };
        order.push(finger);
    }
    Ok((mapping, order))
}

fn parse_alt_fingers(path: &Path, value: Option<&Value>) -> Result<Vec<AltRule>> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let seq = value
        .as_sequence()
        .ok_or_else(|| fail(path, "alt_fingers", "expected a list"))?;
    let mut out = Vec::new();
    for (i, item) in seq.iter().enumerate() {
        let where_i = format!("alt_fingers[{}]", i);
        let map = as_mapping(path, &where_i, item)?;
        let keys = parse_alt_ngram(
            path,
            &format!("{}.ngram", where_i),
            get(map, "ngram").ok_or_else(|| fail(path, &where_i, "missing ngram"))?,
        )?;
        let finger_values: Vec<&Value> = match get(map, "fingers") {
            Some(Value::Sequence(items)) => items.iter().collect(),
            Some(value) => vec![value],
            None => return Err(fail(path, &where_i, "missing fingers")),
        };
        if finger_values.len() != keys.len() {
            return Err(fail(
                path,
                &format!("{}.fingers", where_i),
                &format!(
                    "expected one finger token per ngram key ({}), got {}",
                    keys.len(),
                    finger_values.len()
                ),
            ));
        }
        let mut fingers = Vec::with_capacity(keys.len());
        for (j, (key, finger)) in keys.iter().zip(finger_values.iter()).enumerate() {
            let where_f = format!("{}.fingers[{}]", where_i, j);
            let index = parse_finger_state(path, &where_f, finger, Some(hand_of_key_index(*key)))?;
            fingers.push(index);
        }
        let mode_text = as_string(
            path,
            &format!("{}.mode", where_i),
            get(map, "mode").unwrap_or(&Value::String("".into())),
        )?;
        let mode = match mode_text.as_str() {
            "allow" => AltMode::Allow,
            "permit" | "license" => AltMode::Permit,
            "block" => AltMode::Block,
            _ => {
                return Err(fail(
                    path,
                    &format!("{}.mode", where_i),
                    "expected allow, permit, or block",
                ))
            }
        };
        out.push(AltRule {
            keys,
            fingers,
            mode,
        });
    }
    Ok(out)
}

fn parse_alt_ngram(path: &Path, where_: &str, value: &Value) -> Result<Vec<char>> {
    let texts: Vec<String> = if let Some(items) = value.as_sequence() {
        items
            .iter()
            .enumerate()
            .map(|(i, item)| as_string(path, &format!("{}[{}]", where_, i), item))
            .collect::<Result<_>>()?
    } else {
        vec![as_string(path, where_, value)?]
    };
    if texts.is_empty() || texts.len() > 3 {
        return Err(fail(
            path,
            where_,
            "expected a unigram, bigram, or trigram of QWERTY key labels",
        ));
    }
    let mut keys = Vec::with_capacity(texts.len());
    for (i, text) in texts.iter().enumerate() {
        let upper = text.to_uppercase();
        let chars: Vec<char> = upper.chars().collect();
        if chars.len() != 1 || key_index(&upper).is_none() {
            return Err(fail(
                path,
                &format!("{}[{}]", where_, i),
                &format!("unknown QWERTY key {text:?}"),
            ));
        }
        keys.push(chars[0]);
    }
    Ok(keys)
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

fn parse_finger_pair_token(path: &Path, where_: &str, value: &Value) -> Result<(usize, usize)> {
    let text = as_string(path, where_, value)?.to_uppercase();
    let chars: Vec<char> = text.chars().collect();
    if chars.len() != 2 || (chars[0] != 'L' && chars[0] != 'R') {
        return Err(fail(
            path,
            where_,
            "expected an explicit hand/finger token (LP/LR/LM/LI/RI/RM/RR/RP)",
        ));
    }
    let fi = finger_index(chars[1]).ok_or_else(|| fail(path, where_, "invalid finger token"))?;
    let hand = if chars[0] == 'L' { 0 } else { 1 };
    Ok((hand, fi))
}

fn parse_finger_pairs(
    path: &Path,
    where_: &str,
    value: Option<&Value>,
) -> Result<Option<Vec<FingerPair>>> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    let seq = value
        .as_sequence()
        .ok_or_else(|| fail(path, where_, "expected a list of two-finger pairs"))?;
    if seq.is_empty() {
        return Ok(None);
    }
    let mut out = Vec::new();
    for (i, item) in seq.iter().enumerate() {
        let pair_where = format!("{}[{}]", where_, i);
        let pair = item
            .as_sequence()
            .ok_or_else(|| fail(path, &pair_where, "expected a two-finger pair"))?;
        if pair.len() != 2 {
            return Err(fail(path, &pair_where, "expected exactly two fingers"));
        }
        let first = parse_finger_pair_token(path, &format!("{}[0]", pair_where), &pair[0])?;
        let second = parse_finger_pair_token(path, &format!("{}[1]", pair_where), &pair[1])?;
        out.push([first, second]);
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
        let mut pairs = None;
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
            pairs = parse_finger_pairs(path, &format!("{}.pairs", where_i), get(map, "pairs"))?;
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
            pairs,
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
    Ok(out)
}

fn parse_redirect_pattern(path: &Path, where_: &str, text: &str) -> Result<[u8; 3]> {
    let parts: Vec<&str> = text.split('-').collect();
    if parts.len() != 3 {
        return Err(fail(
            path,
            where_,
            &format!("expected three 1-4 numbers separated by '-', got {text:?}"),
        ));
    }
    let mut pattern = [0u8; 3];
    for (i, part) in parts.iter().enumerate() {
        let bytes = part.as_bytes();
        if bytes.len() != 1 || !(b'1'..=b'4').contains(&bytes[0]) {
            return Err(fail(
                path,
                where_,
                &format!("expected a finger number from 1 to 4, got {part:?}"),
            ));
        }
        pattern[i] = bytes[0] - b'0';
    }
    Ok(pattern)
}

fn parse_redirect_tiers(path: &Path, value: Option<&Value>) -> Result<Option<RedirectTiers>> {
    let Some(value) = value else {
        return Ok(None);
    };
    let seq = value
        .as_sequence()
        .ok_or_else(|| fail(path, "redirect_tiers", "expected a non-empty list of tiers"))?;
    if seq.is_empty() {
        return Err(fail(
            path,
            "redirect_tiers",
            "expected a non-empty list of tiers",
        ));
    }
    let mut seen = HashSet::new();
    let mut tiers = Vec::new();
    for (i, item) in seq.iter().enumerate() {
        let where_i = format!("redirect_tiers[{}]", i);
        let patterns = item.as_sequence().ok_or_else(|| {
            fail(
                path,
                &where_i,
                "expected a non-empty list of redirect patterns",
            )
        })?;
        if patterns.is_empty() {
            return Err(fail(
                path,
                &where_i,
                "expected a non-empty list of redirect patterns",
            ));
        }
        let mut tier = Vec::new();
        for (j, item) in patterns.iter().enumerate() {
            let where_j = format!("{}[{}]", where_i, j);
            let text = as_string(path, &where_j, item)?;
            let pattern = parse_redirect_pattern(path, &where_j, &text)?;
            if pattern[0] == pattern[1] && pattern[1] == pattern[2] {
                return Err(fail(
                    path,
                    &where_j,
                    &format!("{text:?} is an SFT, not a redirect"),
                ));
            }
            if !is_turn(pattern) {
                return Err(fail(
                    path,
                    &where_j,
                    &format!("{text:?} is not a same-hand direction reversal (for example 1-2-1)"),
                ));
            }
            if !seen.insert(pattern) {
                return Err(fail(
                    path,
                    &where_j,
                    &format!("duplicate redirect pattern {text:?}"),
                ));
            }
            tier.push(pattern);
        }
        tiers.push(tier);
    }
    Ok(Some(RedirectTiers::new(tiers)))
}

fn parse_roll_pair(path: &Path, where_: &str, text: &str) -> Result<[u8; 2]> {
    let parts: Vec<&str> = text.split('-').collect();
    if parts.len() != 2 {
        return Err(fail(
            path,
            where_,
            &format!("expected two 1-4 numbers separated by '-', got {text:?}"),
        ));
    }
    let mut pair = [0u8; 2];
    for (i, part) in parts.iter().enumerate() {
        let bytes = part.as_bytes();
        if bytes.len() != 1 || !(b'1'..=b'4').contains(&bytes[0]) {
            return Err(fail(
                path,
                where_,
                &format!("expected a finger number from 1 to 4, got {part:?}"),
            ));
        }
        pair[i] = bytes[0] - b'0';
    }
    Ok(pair)
}

fn parse_roll_tiers(path: &Path, value: Option<&Value>) -> Result<Option<RollTiers>> {
    let Some(value) = value else {
        return Ok(None);
    };
    let seq = value
        .as_sequence()
        .ok_or_else(|| fail(path, "roll_tiers", "expected a non-empty list of tiers"))?;
    if seq.is_empty() {
        return Err(fail(
            path,
            "roll_tiers",
            "expected a non-empty list of tiers",
        ));
    }
    let mut seen = HashSet::new();
    let mut tiers = Vec::new();
    for (i, item) in seq.iter().enumerate() {
        let where_i = format!("roll_tiers[{}]", i);
        let pairs = item
            .as_sequence()
            .ok_or_else(|| fail(path, &where_i, "expected a non-empty list of roll pairs"))?;
        if pairs.is_empty() {
            return Err(fail(
                path,
                &where_i,
                "expected a non-empty list of roll pairs",
            ));
        }
        let mut tier = Vec::new();
        for (j, item) in pairs.iter().enumerate() {
            let where_j = format!("{}[{}]", where_i, j);
            let text = as_string(path, &where_j, item)?;
            let pair = parse_roll_pair(path, &where_j, &text)?;
            if pair[0] == pair[1] {
                return Err(fail(
                    path,
                    &where_j,
                    &format!("{text:?} is an SFB, not a roll"),
                ));
            }
            if !is_adjacent(pair) {
                return Err(fail(
                    path,
                    &where_j,
                    &format!(
                        "{text:?} is not a roll between neighbouring fingers (for example 1-2)"
                    ),
                ));
            }
            if !seen.insert(pair) {
                return Err(fail(
                    path,
                    &where_j,
                    &format!("duplicate roll pair {text:?}"),
                ));
            }
            tier.push(pair);
        }
        tiers.push(tier);
    }
    Ok(Some(RollTiers::new(tiers)))
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

fn parse_finger_strength(path: &Path, value: Option<&Value>) -> Result<[f64; NF]> {
    let mut out = [0.0; NF];
    let Some(value) = value else {
        return Ok(out);
    };
    let map = as_mapping(path, "finger_strength", value)?;
    for (k, v) in map {
        let key = as_string(path, "finger_strength key", k)?;
        let chars: Vec<char> = key.to_uppercase().chars().collect();
        let (hand, finger) = match chars.as_slice() {
            [finger] => (None, FINGERS.iter().position(|f| f == finger)),
            [hand, finger] if *hand == 'L' || *hand == 'R' => (
                Some(if *hand == 'L' { 0 } else { 1 }),
                FINGERS.iter().position(|f| f == finger),
            ),
            _ => (None, None),
        };
        let Some(finger) = finger else {
            return Err(fail(
                path,
                &format!("finger_strength.{}", key),
                "expected LP/LR/LM/LI/RP/RR/RM/RI or P/R/M/I",
            ));
        };
        let number = as_number(
            path,
            &format!("finger_strength.{}", key),
            v,
            Some(0.0),
            None,
        )?;
        match hand {
            Some(hand) => out[hand * NFINGER + finger] = number,
            None => {
                out[finger] = number;
                out[NFINGER + finger] = number;
            }
        }
    }
    Ok(out)
}

fn parse_balance_source(path: &Path, value: Option<&Value>) -> Result<BalanceSource> {
    let Some(value) = value else {
        return Ok(BalanceSource::Chosen);
    };
    let text = as_string(path, "finger_balance_source", value)?;
    match text.to_lowercase().as_str() {
        "chosen" | "flexible" => Ok(BalanceSource::Chosen),
        "default" | "fingermap" | "fixed" => Ok(BalanceSource::Default),
        other => Err(fail(
            path,
            "finger_balance_source",
            &format!("unknown source {other:?}; expected \"chosen\" or \"default\""),
        )),
    }
}

fn parse_hand_chars(path: &Path, where_: &str, value: &Value) -> Result<Vec<char>> {
    let mut out = Vec::new();
    match value {
        Value::Null => {}
        Value::String(text) => {
            out.extend(text.chars().filter(|c| !c.is_whitespace() && *c != ','));
        }
        Value::Sequence(seq) => {
            for (i, item) in seq.iter().enumerate() {
                let text = as_string(path, &format!("{}[{}]", where_, i), item)?;
                out.extend(text.chars().filter(|c| !c.is_whitespace() && *c != ','));
            }
        }
        _ => {
            return Err(fail(
                path,
                where_,
                "expected a string or list of characters",
            ))
        }
    }
    Ok(out)
}

fn parse_hand_restrictions(path: &Path, value: Option<&Value>) -> Result<[Option<usize>; 26]> {
    let mut out = [None; 26];
    let Some(value) = value else {
        return Ok(out);
    };
    if value.is_null() {
        return Ok(out);
    }
    let map = as_mapping(path, "hand_restrictions", value)?;
    for (key, val) in map {
        let name = as_string(path, "hand_restrictions key", key)?;
        let hand = match name.trim().to_ascii_lowercase().as_str() {
            "left" | "l" => 0,
            "right" | "r" => 1,
            _ => {
                return Err(fail(
                    path,
                    &format!("hand_restrictions.{}", name),
                    "expected left or right",
                ))
            }
        };
        let where_ = format!("hand_restrictions.{}", name);
        for ch in parse_hand_chars(path, &where_, val)? {
            let lower = ch.to_ascii_lowercase();
            if !lower.is_ascii_alphabetic() {
                return Err(fail(
                    path,
                    &where_,
                    &format!("expected an ASCII letter, got {ch:?}"),
                ));
            }
            let idx = (lower as u8 - b'a') as usize;
            if out[idx].is_some() {
                return Err(fail(
                    path,
                    &where_,
                    &format!("character {lower:?} is restricted more than once"),
                ));
            }
            out[idx] = Some(hand);
        }
    }
    Ok(out)
}

const DEFAULT_CONDITIONS_YAML: &str = r#"
- {type: scissors, distance: full, reverse: true, fingers: [LP, LR, LM, RM, RR, RP], allow: false}
- {type: scissors, distance: half, fingers: [LP, RP], allow: false}
- {type: scissors, distance: half, allow: true}
- {type: spread, max_extra: 2.0, allow: false}
- {type: separation, min_width: 1.9, fingers: [LP, LI], allow: false}
- {type: crossing, deficit: 0.30, allow: false}
"#;

fn default_conditions() -> Result<Value> {
    serde_yaml::from_str(DEFAULT_CONDITIONS_YAML).context("default conditions")
}

fn parse_scissor_tiers(path: &Path, value: Option<&Value>) -> Result<Option<ScissorTiers>> {
    let Some(value) = value else {
        return Ok(None);
    };
    let seq = value
        .as_sequence()
        .ok_or_else(|| fail(path, "scissor_tiers", "expected a non-empty list of bands"))?;
    if seq.is_empty() {
        return Err(fail(
            path,
            "scissor_tiers",
            "expected a non-empty list of bands",
        ));
    }
    let mut multipliers = [1.0; 4];
    let mut seen = [false; 4];
    for (i, item) in seq.iter().enumerate() {
        let where_i = format!("scissor_tiers[{}]", i);
        let map = as_mapping(path, &where_i, item)?;
        for key in map.keys() {
            let name = key.as_str().unwrap_or("?");
            if !["band", "multiplier"].contains(&name) {
                return Err(fail(
                    path,
                    &where_i,
                    &format!("unknown field {name:?}; expected band or multiplier"),
                ));
            }
        }
        let name = as_string(
            path,
            &format!("{where_i}.band"),
            get(map, "band").ok_or_else(|| fail(path, &where_i, "missing band"))?,
        )?;
        let band = SCISSOR_BANDS
            .iter()
            .position(|candidate| *candidate == name)
            .ok_or_else(|| {
                fail(
                    path,
                    &format!("{where_i}.band"),
                    &format!("unknown band {name:?}; expected one of {:?}", SCISSOR_BANDS),
                )
            })?;
        if seen[band] {
            return Err(fail(
                path,
                &format!("{where_i}.band"),
                &format!("duplicate scissor band {name:?}"),
            ));
        }
        let multiplier = as_number(
            path,
            &format!("{where_i}.multiplier"),
            get(map, "multiplier").ok_or_else(|| fail(path, &where_i, "missing multiplier"))?,
            Some(0.0),
            None,
        )?;
        multipliers[band] = multiplier;
        seen[band] = true;
    }
    Ok(Some(ScissorTiers::new(multipliers)))
}

pub fn load(path: Option<&Path>) -> Result<Config> {
    let path = path
        .map(Path::to_path_buf)
        .unwrap_or_else(paths::config_file);
    let text = paths::read_to_string(&path)?;
    let raw: Value =
        serde_yaml::from_str(&text).with_context(|| format!("{}: invalid YAML", path.display()))?;
    let root = as_mapping(&path, "root", &raw)?;
    let required = [
        "general",
        "model",
        "fingermap",
        "alt_fingers",
        "conditions",
        "priority_list",
        "finger_balance",
    ];
    let known = [
        required.as_slice(),
        &[
            "hand_restrictions",
            "redirect_tiers",
            "redirect_scale",
            "roll_tiers",
            "scissor_tiers",
            "finger_strength",
            "finger_balance_source",
        ],
    ]
    .concat();
    for key in root.keys() {
        let name = key.as_str().unwrap_or("?");
        if !known.contains(&name) {
            return Err(fail(&path, "root", &format!("unknown section {name:?}")));
        }
    }
    for section in required {
        if get(root, section).is_none() {
            return Err(fail(
                &path,
                "root",
                &format!("missing required section {section:?}"),
            ));
        }
    }

    let general = as_mapping(&path, "general", get(root, "general").unwrap())?;
    let geometry_value = as_string(
        &path,
        "general.geometry",
        get(general, "geometry").unwrap_or(&Value::String("".into())),
    )?;
    let geometry = {
        let p = PathBuf::from(shellexpand_home(&geometry_value));
        if p.is_absolute() {
            p
        } else {
            paths::repo_root().join("shirolyzer").join(p)
        }
    };
    if !geometry.is_file() {
        return Err(fail(
            &path,
            "general.geometry",
            &format!("geometry file not found: {}", geometry.display()),
        ));
    }
    let punctuation_text = as_string(
        &path,
        "general.punctuation",
        get(general, "punctuation").unwrap_or(&Value::String("".into())),
    )?;
    let punctuation: Vec<char> = punctuation_text
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    if punctuation.len() != NK - 26 {
        return Err(fail(
            &path,
            "general.punctuation",
            &format!(
                "expected {} punctuation characters, got {}",
                NK - 26,
                punctuation.len()
            ),
        ));
    }
    if punctuation.iter().any(|c| c.is_ascii_alphabetic()) {
        return Err(fail(
            &path,
            "general.punctuation",
            "punctuation must not contain letters",
        ));
    }
    let mut unique = std::collections::HashSet::new();
    if !punctuation.iter().all(|c| unique.insert(*c)) {
        return Err(fail(
            &path,
            "general.punctuation",
            "punctuation characters must be distinct",
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
    let primary_sacrifice = parse_sacrifice(&path, get(general, "primary_sacrifice"))?;
    let fingering = parse_fingering(&path, get(general, "fingering"))?;

    let model = as_mapping(&path, "model", get(root, "model").unwrap())?;
    let mut disallow_reverse_scissors = [[false; NFINGER]; NFINGER];
    if let Some(value) = get(model, "disallow_reverse_scissors") {
        let tokens: Vec<&Value> = if let Some(seq) = value.as_sequence() {
            seq.iter().collect()
        } else {
            vec![value]
        };
        for (i, token) in tokens.iter().enumerate() {
            let text = as_string(
                &path,
                &format!("model.disallow_reverse_scissors[{}]", i),
                token,
            )?
            .to_uppercase();
            let chars: Vec<char> = text.chars().collect();
            if chars.len() != 2
                || chars[0] == chars[1]
                || !FINGERS.contains(&chars[0])
                || !FINGERS.contains(&chars[1])
            {
                return Err(fail(
                    &path,
                    &format!("model.disallow_reverse_scissors[{}]", i),
                    "expected two distinct finger letters from P/R/M/I (for example MR)",
                ));
            }
            let mut pair = [
                finger_index(chars[0]).unwrap(),
                finger_index(chars[1]).unwrap(),
            ];
            pair.sort_unstable();
            disallow_reverse_scissors[pair[0]][pair[1]] = true;
        }
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
    let redirect_tiers = parse_redirect_tiers(&path, get(root, "redirect_tiers"))?;
    let redirect_scale = match get(root, "redirect_scale") {
        Some(value) => as_number(&path, "redirect_scale", value, Some(0.0), None)?,
        None => 1.0,
    };
    if redirect_scale <= 0.0 {
        return Err(fail(
            &path,
            "redirect_scale",
            "must be greater than 0 (to switch redirects off, leave them out of priority_list)",
        ));
    }
    let roll_tiers = parse_roll_tiers(&path, get(root, "roll_tiers"))?;
    let scissor_tiers = parse_scissor_tiers(&path, get(root, "scissor_tiers"))?;
    let (finger_balance, flat_share) = parse_balance(&path, get(root, "finger_balance"))?;
    let finger_strength = parse_finger_strength(&path, get(root, "finger_strength"))?;
    let balance_source = parse_balance_source(&path, get(root, "finger_balance_source"))?;
    let hand_restrictions = parse_hand_restrictions(&path, get(root, "hand_restrictions"))?;
    let conditions_are_default = match default_conditions() {
        Ok(default) => default == *get(root, "conditions").unwrap_or(&Value::Null),
        Err(_) => false,
    };

    Ok(Config {
        path,
        raw,
        geometry,
        punctuation,
        primary_corpus,
        secondary_corpus,
        primary_sacrifice,
        disallow_reverse_scissors,
        parameters_file,
        parameters_path,
        fingering,
        fingermap,
        finger_tokens,
        alt_fingers,
        conditions,
        conditions_are_default,
        priority_list,
        redirect_tiers,
        redirect_scale,
        roll_tiers,
        scissor_tiers,
        finger_balance,
        flat_share,
        finger_strength,
        balance_source,
        hand_restrictions,
    })
}

/// Build the model from a loaded config.
pub fn build_model(cfg: &Config) -> Result<crate::model::Model> {
    let mut model = crate::model::Model::from_file(
        &cfg.parameters_path,
        cfg.disallow_reverse_scissors,
        cfg.fingermap,
    )?;
    model.scales = cfg.scales();
    model.finger_strength = cfg.finger_strength;
    Ok(model)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tiers(yaml: &str) -> Vec<Vec<String>> {
        let value: Value = serde_yaml::from_str(yaml).unwrap();
        parse_priority(Path::new("config.yaml"), Some(&value)).unwrap()
    }

    #[test]
    fn priority_tiers_are_weighted_from_the_top() {
        let list = tiers("[sfbs-sfts, scissors, redirects, [rolls, finger_balance], alternation]");
        let w = priority_weights(&list);
        assert_eq!(w["sfbs-sfts"], 3.0);
        assert_eq!(w["scissors"], 2.5);
        assert_eq!(w["redirects"], 2.0);
        assert_eq!(w["rolls"], 1.5);
        assert_eq!(w["finger_balance"], 1.5);
        assert_eq!(w["alternation"], 1.0);
        assert!(omitted_priorities(&list).is_empty());
    }

    #[test]
    fn the_old_default_ordering_is_no_longer_a_silent_no_op() {
        // This exact list used to be special-cased as "unweighted".
        let list =
            tiers("[sfbs-sfts, [scissors, redirects], [rolls, alternation], finger_balance]");
        let w = priority_weights(&list);
        assert_eq!(w["sfbs-sfts"], 2.5);
        assert_eq!(w["scissors"], 2.0);
        assert_eq!(w["redirects"], 2.0);
        assert_eq!(w["rolls"], 1.5);
        assert_eq!(w["alternation"], 1.5);
        assert_eq!(w["finger_balance"], 1.0);
    }

    #[test]
    fn a_single_tier_weights_everything_equally() {
        let list = tiers("[[sfbs-sfts, scissors, redirects, rolls, alternation, finger_balance]]");
        assert!(priority_weights(&list).values().all(|v| *v == 1.0));
    }

    #[test]
    fn omitted_priorities_are_switched_off_and_reported() {
        let list = tiers("[sfbs-sfts, [scissors, redirects], [rolls, alternation]]");
        let w = priority_weights(&list);
        assert_eq!(w["finger_balance"], 0.0);
        assert_eq!(w["sfbs-sfts"], 2.0);
        assert_eq!(omitted_priorities(&list), vec!["finger_balance"]);
    }

    fn restrictions(yaml: &str) -> Result<[Option<usize>; 26]> {
        let value: Value = serde_yaml::from_str(yaml).unwrap();
        parse_hand_restrictions(Path::new("config.yaml"), Some(&value))
    }

    #[test]
    fn hand_restrictions_accept_lists_and_strings() {
        let parsed = restrictions("left: [z, X]\nright: b\n").unwrap();
        assert_eq!(parsed[25], Some(0));
        assert_eq!(parsed[23], Some(0));
        assert_eq!(parsed[1], Some(1));
        assert_eq!(parsed[0], None);

        let parsed = restrictions("left: zx\nright: [b]\n").unwrap();
        assert_eq!(parsed[25], Some(0));
        assert_eq!(parsed[23], Some(0));
        assert_eq!(parsed[1], Some(1));
    }

    #[test]
    fn hand_restrictions_are_optional() {
        let parsed = parse_hand_restrictions(Path::new("config.yaml"), None).unwrap();
        assert!(parsed.iter().all(Option::is_none));
    }

    #[test]
    fn hand_restrictions_reject_conflicts() {
        assert!(restrictions("left: z\nright: z\n").is_err());
        assert!(restrictions("left: [z, z]\n").is_err());
        assert!(restrictions("middle: z\n").is_err());
        assert!(restrictions("left: [z, 1]\n").is_err());
    }

    #[test]
    fn fingering_mode_defaults_to_flexible_and_parses_aliases() {
        let path = Path::new("config.yaml");
        assert_eq!(
            parse_fingering(path, None).unwrap(),
            FingeringMode::Flexible
        );
        for text in ["fixed", "default", "FIXED"] {
            let value = Value::String(text.into());
            assert_eq!(
                parse_fingering(path, Some(&value)).unwrap(),
                FingeringMode::Fixed,
                "{text}"
            );
        }
        for text in ["flexible", "free", "Flexible"] {
            let value = Value::String(text.into());
            assert_eq!(
                parse_fingering(path, Some(&value)).unwrap(),
                FingeringMode::Flexible,
                "{text}"
            );
        }
        let bad = Value::String("contextual".into());
        assert!(parse_fingering(path, Some(&bad)).is_err());
    }
}

#[cfg(test)]
mod cross_hand_tests {
    use super::*;
    use crate::geometry::{finger_hand, hand_of, parse_finger8};

    #[test]
    fn alt_fingers_accept_explicit_cross_hand_tokens() {
        let value: Value = serde_yaml::from_str(
            r#"
- {ngram: [H], fingers: [LI], mode: allow}
- {ngram: [Y], fingers: [LM], mode: allow}
- {ngram: [H], fingers: [I], mode: allow}
"#,
        )
        .unwrap();
        let rules = parse_alt_fingers(Path::new("config.yaml"), Some(&value)).unwrap();
        assert_eq!(rules[0].fingers, vec![parse_finger8("LI").unwrap()]);
        assert_eq!(rules[1].fingers, vec![parse_finger8("LM").unwrap()]);
        // A bare token still means the physical key's own hand.
        assert_eq!(rules[2].fingers, vec![parse_finger8("RI").unwrap()]);
    }

    #[test]
    fn alt_fingers_reject_invalid_tokens() {
        let value: Value =
            serde_yaml::from_str("- {ngram: [H], fingers: [LX], mode: allow}\n").unwrap();
        assert!(parse_alt_fingers(Path::new("config.yaml"), Some(&value)).is_err());
    }

    #[test]
    fn shipped_fingermap_stays_same_hand() {
        let cfg = load(None).unwrap();
        for (key, &finger) in KEYS.iter().zip(cfg.fingermap.iter()) {
            assert_eq!(
                finger_hand(finger),
                hand_of(key),
                "{key} uses a cross-hand default"
            );
        }
        // Cross-hand tokens are legal in alt rules, but the canonical fixed
        // config must not opt into them by accident.
        let fixed = load(Some(&paths::config_dir().join("config.fixed.yaml"))).unwrap();
        for rule in &fixed.alt_fingers {
            for (&key, &finger) in rule.keys.iter().zip(rule.fingers.iter()) {
                let key = key.to_string();
                if finger_hand(finger) != hand_of(&key) {
                    panic!("fixed config alt rule {key} uses cross-hand {finger}");
                }
            }
        }
    }

    #[test]
    fn parse_finger_state_requires_explicit_hand_without_default() {
        let value = Value::String("I".into());
        assert!(parse_finger_state(Path::new("config.yaml"), "fingers[0]", &value, None).is_err());
        assert_eq!(
            parse_finger_state(Path::new("config.yaml"), "fingers[0]", &value, Some(0)).unwrap(),
            parse_finger8("LI").unwrap()
        );
    }
    fn redirects(yaml: &str) -> Result<Option<RedirectTiers>> {
        let value: Value = serde_yaml::from_str(yaml).unwrap();
        parse_redirect_tiers(Path::new("config.yaml"), Some(&value))
    }

    #[test]
    fn shipped_redirect_tiers_preserve_order() {
        let cfg = load(None).unwrap();
        let tiers = cfg.redirect_tiers.as_ref().expect("redirect tiers");
        let expected: [&[[u8; 3]]; 5] = [
            &[[1, 2, 1], [2, 1, 2], [1, 3, 1], [3, 1, 3]],
            &[[3, 2, 3]],
            &[[2, 1, 3], [3, 1, 2], [2, 3, 2], [1, 4, 1]],
            &[[4, 1, 4], [4, 2, 4], [2, 4, 2]],
            &[[3, 4, 3], [4, 3, 4], [3, 2, 4], [4, 2, 3]],
        ];
        assert_eq!(tiers.len(), expected.len());
        for (tier, patterns) in expected.iter().enumerate() {
            assert_eq!(tiers.tiers()[tier], patterns.to_vec());
            for &pattern in patterns.iter() {
                assert_eq!(tiers.tier_of(pattern), Some(tier));
            }
        }
        for pattern in [
            [1, 3, 2],
            [1, 4, 2],
            [1, 4, 3],
            [2, 1, 4],
            [2, 3, 1],
            [2, 4, 1],
            [2, 4, 3],
            [3, 1, 4],
            [3, 4, 1],
            [3, 4, 2],
            [4, 1, 2],
            [4, 1, 3],
        ] {
            assert!(is_turn(pattern));
            assert_eq!(tiers.tier_of(pattern), None);
        }
    }

    #[test]
    fn redirect_tiers_are_optional() {
        let parsed = parse_redirect_tiers(Path::new("config.yaml"), None).unwrap();
        assert!(parsed.is_none());
    }

    #[test]
    fn redirect_tiers_accept_the_documented_shape() {
        let parsed = redirects(
            r#"
- ["1-2-1", "2-1-2"]
- ["3-2-3"]
"#,
        )
        .unwrap()
        .unwrap();
        assert_eq!(parsed.tiers()[0], vec![[1, 2, 1], [2, 1, 2]]);
        assert_eq!(parsed.tiers()[1], vec![[3, 2, 3]]);
    }

    #[test]
    fn redirect_tiers_reject_malformed_entries() {
        for yaml in [
            "[]",
            "- []",
            "- [1, 2, 1]",
            r#"- ["1-2"]"#,
            r#"- ["1-2-3-4"]"#,
            r#"- ["1-2-a"]"#,
            r#"- ["0-2-1"]"#,
            r#"- ["5-2-1"]"#,
            r#"- ["01-2-1"]"#,
            r#"- ["1-2-1", "1-2-1"]"#,
            "- [\"1-2-1\"]\n- [\"1-2-1\"]",
            r#"- ["1-2-3"]"#,
            r#"- ["2-2-2"]"#,
        ] {
            assert!(redirects(yaml).is_err(), "{yaml} should fail");
        }
    }

    fn rolls(yaml: &str) -> Result<Option<RollTiers>> {
        let value: Value = serde_yaml::from_str(yaml).unwrap();
        parse_roll_tiers(Path::new("config.yaml"), Some(&value))
    }

    #[test]
    fn roll_tiers_are_optional() {
        let parsed = parse_roll_tiers(Path::new("config.yaml"), None).unwrap();
        assert!(parsed.is_none());
    }

    #[test]
    fn roll_tiers_accept_the_documented_shape() {
        let parsed = rolls(
            r#"
- ["1-2"]
- ["2-1", "3-2"]
"#,
        )
        .unwrap()
        .unwrap();
        assert_eq!(parsed.tiers()[0], vec![[1, 2]]);
        assert_eq!(parsed.tiers()[1], vec![[2, 1], [3, 2]]);
        assert_eq!(parsed.tier_of([3, 2]), Some(1));
        assert_eq!(parsed.tier_of([2, 3]), None);
    }

    #[test]
    fn roll_tiers_reject_malformed_entries() {
        for yaml in [
            "[]",
            "- []",
            "- [1, 2]",
            r#"- ["1"]"#,
            r#"- ["1-2-1"]"#,
            r#"- ["1-a"]"#,
            r#"- ["0-1"]"#,
            r#"- ["5-4"]"#,
            r#"- ["01-2"]"#,
            r#"- ["1-2", "1-2"]"#,
            "- [\"1-2\"]\n- [\"1-2\"]",
            r#"- ["1-3"]"#,
            r#"- ["4-1"]"#,
            r#"- ["2-2"]"#,
        ] {
            assert!(rolls(yaml).is_err(), "{yaml} should fail");
        }
    }

    #[test]
    fn roll_tiers_reach_the_model_scales() {
        let mut cfg = load(None).unwrap();
        cfg.roll_tiers = rolls(r#"- ["1-2"]"#).unwrap();
        let table = cfg.scales().roll_credits.expect("roll credits");
        assert_eq!(table.credit([1, 2]), Some(crate::model::W_ROLL_TIER_BEST));
        assert_eq!(table.credit([2, 1]), None);
        cfg.roll_tiers = None;
        assert!(cfg.scales().roll_credits.is_none());
    }

    fn scissors(yaml: &str) -> Result<Option<ScissorTiers>> {
        let value: Value = serde_yaml::from_str(yaml).unwrap();
        parse_scissor_tiers(Path::new("config.yaml"), Some(&value))
    }

    #[test]
    fn scissor_tiers_are_optional() {
        let parsed = parse_scissor_tiers(Path::new("config.yaml"), None).unwrap();
        assert!(parsed.is_none());
    }

    #[test]
    fn scissor_tiers_accept_the_documented_shape() {
        let parsed = scissors(
            r#"
- {band: mild, multiplier: 0.5}
- {band: moderate, multiplier: 0.75}
- {band: severe, multiplier: 1.0}
- {band: extreme, multiplier: 0.1}
"#,
        )
        .unwrap()
        .unwrap();
        assert_eq!(parsed.multipliers(), [0.5, 0.75, 1.0, 0.1]);
    }

    #[test]
    fn scissor_tiers_may_leave_bands_at_the_default() {
        let parsed = scissors(r#"- {band: mild, multiplier: 0.5}"#)
            .unwrap()
            .unwrap();
        assert_eq!(parsed.multipliers(), [0.5, 1.0, 1.0, 1.0]);
    }

    #[test]
    fn scissor_tiers_reject_malformed_entries() {
        for yaml in [
            "[]",
            "- mild",
            "- {band: mild}",
            "- {multiplier: 0.5}",
            "- {band: tiny, multiplier: 0.5}",
            "- {band: mild, multiplier: -0.1}",
            "- {band: mild, multiplier: lots}",
            "- {band: mild, multiplier: 0.5, extra: 1}",
            "- {band: mild, multiplier: 0.5}\n- {band: mild, multiplier: 0.5}",
        ] {
            assert!(scissors(yaml).is_err(), "{yaml} should fail");
        }
    }

    #[test]
    fn scissor_tiers_reach_the_model_scales() {
        let mut cfg = load(None).unwrap();
        cfg.scissor_tiers = scissors(
            r#"
- {band: mild, multiplier: 0.5}
- {band: moderate, multiplier: 0.75}
- {band: severe, multiplier: 1.0}
- {band: extreme, multiplier: 0.1}
"#,
        )
        .unwrap();
        let table = cfg.scales().scissor_tiers.expect("scissor tiers");
        assert_eq!(table.multipliers(), [0.5, 0.75, 1.0, 0.1]);
        cfg.scissor_tiers = None;
        assert!(cfg.scales().scissor_tiers.is_none());
    }

    #[test]
    fn shipped_config_sets_scissor_tiers() {
        let cfg = load(None).unwrap();
        let tiers = cfg.scissor_tiers.as_ref().expect("scissor tiers");
        assert_eq!(tiers.multipliers(), [0.5, 0.75, 1.0, 0.12]);
    }

    #[test]
    fn finger_strength_is_optional_and_parses_bare_and_explicit_tokens() {
        let path = Path::new("config.yaml");
        assert_eq!(parse_finger_strength(path, None).unwrap(), [0.0; NF]);
        let value: Value = serde_yaml::from_str("P: 0.4\nRI: 0.2\n").unwrap();
        let strength = parse_finger_strength(path, Some(&value)).unwrap();
        assert_eq!(strength[0], 0.4);
        assert_eq!(strength[4], 0.4);
        assert_eq!(strength[7], 0.2);
        assert_eq!(strength[3], 0.0);
    }

    #[test]
    fn finger_strength_rejects_bad_tokens_and_negative_values() {
        for yaml in ["LX: 1\n", "Z: 1\n", "P: -1\n", "P: lots\n"] {
            let value: Value = serde_yaml::from_str(yaml).unwrap();
            assert!(
                parse_finger_strength(Path::new("config.yaml"), Some(&value)).is_err(),
                "{yaml}"
            );
        }
    }

    #[test]
    fn balance_source_defaults_to_chosen_and_parses_aliases() {
        let path = Path::new("config.yaml");
        assert_eq!(
            parse_balance_source(path, None).unwrap(),
            BalanceSource::Chosen
        );
        for (text, expected) in [
            ("chosen", BalanceSource::Chosen),
            ("flexible", BalanceSource::Chosen),
            ("default", BalanceSource::Default),
            ("fingermap", BalanceSource::Default),
        ] {
            let value = Value::String(text.into());
            assert_eq!(parse_balance_source(path, Some(&value)).unwrap(), expected);
        }
        let bad = Value::String("mixed".into());
        assert!(parse_balance_source(path, Some(&bad)).is_err());
    }

    fn config_with(extra: &str) -> Result<Config> {
        let text = std::fs::read_to_string(crate::paths::config_file()).unwrap();
        let dir = std::env::temp_dir().join(format!("shirolyzer-config-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(format!("c{}.yaml", extra.len()));
        let mut text: String = text
            .lines()
            .filter(|l| !l.starts_with("redirect_scale:"))
            .collect::<Vec<_>>()
            .join("\n");
        text.push('\n');
        text.push_str(extra);
        std::fs::write(&path, text).unwrap();
        load(Some(&path))
    }

    #[test]
    fn shipped_config_sets_roll_tiers_and_redirect_scale() {
        let cfg = load(None).unwrap();
        let rolls = cfg.roll_tiers.as_ref().expect("roll tiers");
        assert_eq!(rolls.tiers(), &[vec![[1, 2], [2, 1]]]);
        assert_eq!(cfg.redirect_scale, 3.0);
        let weights = cfg.priority_multipliers();
        assert_eq!(cfg.scales().redirect, weights["redirects"] * 3.0);
        assert!(cfg.priority_summary().contains("redirect_scale ×3"));
    }

    #[test]
    fn redirect_scale_defaults_to_one_and_rejects_non_positive_values() {
        assert_eq!(config_with("").unwrap().redirect_scale, 1.0);
        assert_eq!(
            config_with("redirect_scale: 1.5\n").unwrap().redirect_scale,
            1.5
        );
        for bad in [
            "redirect_scale: 0\n",
            "redirect_scale: -2\n",
            "redirect_scale: lots\n",
        ] {
            assert!(config_with(bad).is_err(), "{bad} should fail");
        }
        let plain = config_with("").unwrap();
        assert!(!plain.priority_summary().contains("redirect_scale"));
    }
}
