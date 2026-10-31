//! ANSI physical geometry: stagger, hand, row, rest posture, reach, and the
//! key/finger displacement tables used by the comfort model.
//!
//! This is a direct port of `shirolyzer/utils/ansi/ansi.py` and
//! `shirolyzer/utils/fingers.py`.  The Python code uses a Delaunay
//! triangulation only as an inside-the-approved-hull test; an explicit convex
//! hull gives the same answer.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{bail, Context, Result};
use serde::Deserialize;

pub const ROWS: [&str; 3] = ["QWERTYUIOP", "ASDFGHJKL;", "ZXCVBNM,./"];
pub const ROW_OFFSET: [f64; 3] = [0.0, 0.25, 0.75];
pub const FINGERS: [char; 4] = ['P', 'R', 'M', 'I'];
/// Number of finger kinds (P/R/M/I) before the hand dimension is added.
pub const NFINGER: usize = 4;
/// Eight finger states in `hand * NFINGER + finger` order.
pub const FINGERS8: [&str; 8] = ["LP", "LR", "LM", "LI", "RP", "RR", "RM", "RI"];
pub const NK: usize = 30;
pub const NF: usize = 8;
pub const CROSS_REACH_MAX: f64 = 4.5;

pub const KEYS: [&str; NK] = [
    "Q", "W", "E", "R", "T", "Y", "U", "I", "O", "P", "A", "S", "D", "F", "G", "H", "J", "K", "L",
    ";", "Z", "X", "C", "V", "B", "N", "M", ",", ".", "/",
];

/// Finger length rank.  Python: `np.array([0, 1, 3, 2])` for P/R/M/I.
pub const LENGTH_RANK: [f64; NFINGER] = [0.0, 1.0, 3.0, 2.0];

pub fn key_index(key: &str) -> Option<usize> {
    KEYS.iter().position(|k| *k == key)
}

pub fn key_index_char(key: char) -> Option<usize> {
    let key = key as u8;
    KEYS.iter().position(|k| k.as_bytes().first() == Some(&key))
}

/// Index of a finger letter (P/R/M/I).
pub fn finger_index(finger: char) -> Option<usize> {
    FINGERS.iter().position(|f| *f == finger)
}

/// Hand of an eight-state finger index.
#[inline]
pub fn finger_hand(finger: usize) -> usize {
    finger / NFINGER
}

/// Finger kind (P/R/M/I) of an eight-state finger index.
#[inline]
pub fn finger_kind(finger: usize) -> usize {
    finger % NFINGER
}

/// Eight-state index for a finger letter on a hand (0 = left, 1 = right).
#[inline]
pub fn finger_index8(finger: char, hand: usize) -> Option<usize> {
    finger_index(finger).map(|kind| hand * NFINGER + kind)
}

/// Parse an explicit hand/finger token such as `LI` or `RM`.
pub fn parse_finger8(token: &str) -> Option<usize> {
    let text = token.to_uppercase();
    let chars: Vec<char> = text.chars().collect();
    if chars.len() != 2 || !matches!(chars[0], 'L' | 'R') {
        return None;
    }
    let hand = if chars[0] == 'L' { 0 } else { 1 };
    finger_index8(chars[1], hand)
}

/// Canonical hand/finger token for an eight-state index.
pub fn finger_token(finger: usize) -> &'static str {
    FINGERS8[finger]
}

#[inline]
pub fn length_rank(finger: usize) -> f64 {
    LENGTH_RANK[finger_kind(finger)]
}

pub fn hand_of(key: &str) -> usize {
    match key_index(key) {
        Some(i) => Hand::default_hand()[i],
        None => 0,
    }
}

#[derive(Debug, Deserialize)]
struct RestSpec {
    left: HashMap<String, String>,
    right: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct GeometrySpec {
    rows: Vec<String>,
    row_offsets: Vec<f64>,
    hand_split: usize,
    rest: RestSpec,
    approved: HashMap<String, Vec<(f64, f64)>>,
    listed: HashMap<String, String>,
}

#[derive(Clone)]
pub struct Geometry {
    pub cx: [f64; NK],
    pub cy: [f64; NK],
    pub hand: [usize; NK],
    pub row: [usize; NK],
    pub u: [f64; NK],
    pub v: [f64; NK],
    pub rest_u: [[f64; NFINGER]; 2],
    pub rest_v: [[f64; NFINGER]; 2],
    pub inner_sep: [[f64; NFINGER]; 2],
    pub u8: [[f64; NK]; NF],
    pub du: [[f64; NK]; NF],
    pub dv: [[f64; NK]; NF],
    pub reach: [[bool; NF]; NK],
    pub cross_reach: [[bool; NF]; NK],
    pub unlisted: [[bool; NF]; NK],
}

impl Default for Geometry {
    fn default() -> Self {
        Self::new()
    }
}

impl Geometry {
    pub fn new() -> Self {
        let path = crate::paths::geometry_file("ansi");
        Self::from_file(&path).unwrap_or_else(|err| panic!("{}", err))
    }

    pub fn from_file(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("{}: read geometry", path.display()))?;
        let spec: GeometrySpec = serde_yaml::from_str(&text)
            .with_context(|| format!("{}: invalid geometry YAML", path.display()))?;
        Self::from_spec(&spec, path)
    }

    fn from_spec(spec: &GeometrySpec, path: &Path) -> Result<Self> {
        if spec.rows.len() != ROWS.len() {
            bail!(
                "{}: geometry.rows must have {} rows, got {}",
                path.display(),
                ROWS.len(),
                spec.rows.len()
            );
        }
        for (i, row) in spec.rows.iter().enumerate() {
            if row != ROWS[i] {
                bail!(
                    "{}: geometry.rows[{}] must be {:?} for this build, got {:?}",
                    path.display(),
                    i,
                    ROWS[i],
                    row
                );
            }
        }
        if spec.row_offsets.len() != ROWS.len() {
            bail!(
                "{}: geometry.row_offsets must have {} values, got {}",
                path.display(),
                ROWS.len(),
                spec.row_offsets.len()
            );
        }
        if spec.hand_split != 5 {
            bail!(
                "{}: geometry.hand_split must be 5 for this build, got {}",
                path.display(),
                spec.hand_split
            );
        }

        let mut cx = [0.0; NK];
        let mut cy = [0.0; NK];
        let mut hand = [0usize; NK];
        let mut row = [0usize; NK];
        let mut idx = 0;
        for (r, row_name) in spec.rows.iter().enumerate() {
            for (c, _ch) in row_name.chars().enumerate() {
                if idx >= NK {
                    bail!("{}: geometry has more than {} keys", path.display(), NK);
                }
                cx[idx] = c as f64 + spec.row_offsets[r] + 0.5;
                cy[idx] = r as f64 + 0.5;
                hand[idx] = if c < spec.hand_split { 0 } else { 1 };
                row[idx] = r;
                idx += 1;
            }
        }
        if idx != NK {
            bail!(
                "{}: geometry has {} keys, expected {}",
                path.display(),
                idx,
                NK
            );
        }

        let mut u = [0.0; NK];
        let mut v = [0.0; NK];
        for i in 0..NK {
            u[i] = if hand[i] == 0 { cx[i] } else { -cx[i] };
            v[i] = cy[i];
        }

        let mut rest_u = [[0.0; NFINGER]; 2];
        let mut rest_v = [[0.0; NFINGER]; 2];
        for (h, map) in [&spec.rest.left, &spec.rest.right].iter().enumerate() {
            for (f, finger) in FINGERS.iter().enumerate() {
                let key = map.get(&finger.to_string()).ok_or_else(|| {
                    anyhow::anyhow!(
                        "{}: geometry.rest.{} is missing finger {}",
                        path.display(),
                        if h == 0 { "left" } else { "right" },
                        finger
                    )
                })?;
                let k = key_index(key).ok_or_else(|| {
                    anyhow::anyhow!(
                        "{}: geometry.rest.{}: unknown key {:?}",
                        path.display(),
                        if h == 0 { "left" } else { "right" },
                        key
                    )
                })?;
                if hand[k] != h {
                    bail!(
                        "{}: geometry.rest key {:?} is not on the {} hand",
                        path.display(),
                        key,
                        if h == 0 { "left" } else { "right" }
                    );
                }
                rest_u[h][f] = u[k];
                rest_v[h][f] = v[k];
            }
        }
        let mut inner_sep = [[0.0; NFINGER]; 2];
        for h in 0..2 {
            for f in 0..NFINGER - 1 {
                inner_sep[h][f] = rest_u[h][f + 1] - rest_u[h][f];
            }
            inner_sep[h][NFINGER - 1] = 99.0;
        }

        let mut u8 = [[0.0; NK]; NF];
        let mut du = [[0.0; NK]; NF];
        let mut dv = [[0.0; NK]; NF];
        for k in 0..NK {
            for f in 0..NF {
                let h = finger_hand(f);
                let kind = finger_kind(f);
                // Same-hand fingers use the existing hand-relative mirrored
                // coordinate.  Cross-hand fingers use the absolute key column,
                // mirrored by the finger's own hand so inner/outer keeps the
                // same sign convention.
                let fu = if h == hand[k] {
                    u[k]
                } else if h == 0 {
                    cx[k]
                } else {
                    -cx[k]
                };
                u8[f][k] = fu;
                du[f][k] = fu - rest_u[h][kind];
                dv[f][k] = v[k] - rest_v[h][kind];
            }
        }

        let mut approved: [Vec<(f64, f64)>; NFINGER] = std::array::from_fn(|_| Vec::new());
        for (f, finger) in FINGERS.iter().enumerate() {
            let points = spec.approved.get(&finger.to_string()).ok_or_else(|| {
                anyhow::anyhow!(
                    "{}: geometry.approved is missing finger {}",
                    path.display(),
                    finger
                )
            })?;
            if points.is_empty() {
                bail!(
                    "{}: geometry.approved.{} must not be empty",
                    path.display(),
                    finger
                );
            }
            approved[f] = points.clone();
        }
        let mut reach = [[false; NF]; NK];
        for (kind, points) in approved.iter().enumerate() {
            let hull = convex_hull(points);
            for k in 0..NK {
                let f = hand[k] * NFINGER + kind;
                reach[k][f] = point_in_hull(&hull, du[f][k], dv[f][k], 1e-9);
            }
        }

        // Cross-hand reach is deliberately separate from the approved
        // same-hand polygons.  It is only a physical bound; gates still keep
        // cross-hand fingerings impossible unless an `alt_fingers` allow rule
        // explicitly licenses them.
        let mut cross_reach = [[false; NF]; NK];
        for k in 0..NK {
            for f in 0..NF {
                if finger_hand(f) == hand[k] {
                    continue;
                }
                cross_reach[k][f] = (du[f][k].powi(2) + dv[f][k].powi(2)).sqrt() <= CROSS_REACH_MAX;
            }
        }

        let mut listed = [[false; NF]; NK];
        for (token, keys) in &spec.listed {
            let chars: Vec<char> = token.to_uppercase().chars().collect();
            if chars.len() != 2 || !matches!(chars[0], 'L' | 'R') || !FINGERS.contains(&chars[1]) {
                bail!(
                    "{}: geometry.listed key {:?} must be LP/LR/LM/LI/RI/RM/RR/RP",
                    path.display(),
                    token
                );
            }
            let h = if chars[0] == 'L' { 0 } else { 1 };
            let f = finger_index(chars[1]).unwrap();
            for key in keys.chars() {
                let k = key_index(&key.to_string()).ok_or_else(|| {
                    anyhow::anyhow!(
                        "{}: geometry.listed.{}: unknown key {:?}",
                        path.display(),
                        token,
                        key
                    )
                })?;
                if hand[k] != h {
                    bail!(
                        "{}: geometry.listed.{} key {:?} is not on the {} hand",
                        path.display(),
                        token,
                        key,
                        if h == 0 { "left" } else { "right" }
                    );
                }
                listed[k][hand[k] * NFINGER + f] = true;
            }
        }
        let mut unlisted = [[false; NF]; NK];
        for k in 0..NK {
            for f in 0..NF {
                if reach[k][f] && !listed[k][f] {
                    unlisted[k][f] = true;
                }
            }
        }
        Ok(Self {
            cx,
            cy,
            hand,
            row,
            u,
            v,
            rest_u,
            rest_v,
            inner_sep,
            u8,
            du,
            dv,
            reach,
            cross_reach,
            unlisted,
        })
    }

    pub fn same_hand(&self, a: usize, b: usize) -> bool {
        self.hand[a] == self.hand[b]
    }

    #[inline]
    pub fn is_reachable(&self, key: usize, finger: usize) -> bool {
        self.reach[key][finger] || self.cross_reach[key][finger]
    }

    pub fn distance(&self, a: usize, b: usize) -> f64 {
        ((self.cx[a] - self.cx[b]).powi(2) + (self.v[a] - self.v[b]).powi(2)).sqrt()
    }
}

/// Convenience for code that only needs the hand array.
pub struct Hand;
impl Hand {
    pub fn default_hand() -> [usize; NK] {
        let mut hand = [0usize; NK];
        let mut idx = 0;
        for (r, row_name) in ROWS.iter().enumerate() {
            let _ = r;
            for (c, _) in row_name.chars().enumerate() {
                hand[idx] = if c < 5 { 0 } else { 1 };
                idx += 1;
            }
        }
        hand
    }
}

fn cross(o: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
    (a.0 - o.0) * (b.1 - o.1) - (a.1 - o.1) * (b.0 - o.0)
}

fn convex_hull(points: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let mut pts = points.to_vec();
    pts.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    pts.dedup();
    if pts.len() <= 1 {
        return pts;
    }
    let mut lower: Vec<(f64, f64)> = Vec::new();
    for &p in &pts {
        while lower.len() >= 2 && cross(lower[lower.len() - 2], lower[lower.len() - 1], p) <= 0.0 {
            lower.pop();
        }
        lower.push(p);
    }
    let mut upper: Vec<(f64, f64)> = Vec::new();
    for &p in pts.iter().rev() {
        while upper.len() >= 2 && cross(upper[upper.len() - 2], upper[upper.len() - 1], p) <= 0.0 {
            upper.pop();
        }
        upper.push(p);
    }
    lower.pop();
    upper.pop();
    lower.extend(upper);
    lower
}

fn point_in_hull(hull: &[(f64, f64)], x: f64, y: f64, eps: f64) -> bool {
    if hull.len() < 3 {
        return false;
    }
    let p = (x, y);
    for i in 0..hull.len() {
        let a = hull[i];
        let b = hull[(i + 1) % hull.len()];
        if cross(a, b, p) < -eps {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cross_hand_travel_uses_absolute_positions() {
        let geom = Geometry::new();
        let h = key_index("H").unwrap();
        let y = key_index("Y").unwrap();
        let li = parse_finger8("LI").unwrap();
        let lm = parse_finger8("LM").unwrap();
        let ri = parse_finger8("RI").unwrap();

        // The default same-hand table stays same-hand only.
        assert!(geom.reach[h][ri]);
        assert!(!geom.reach[h][li]);

        // Explicit cross-hand states are physically reachable and priced from
        // absolute key/finger columns rather than a hand-relative mirror.
        assert!(geom.cross_reach[h][li]);
        assert!(geom.is_reachable(h, li));
        assert!(geom.is_reachable(y, lm));
        assert!((geom.du[li][h] - (geom.cx[h] - geom.rest_u[0][3])).abs() < 1e-9);
        assert!((geom.du[lm][y] - (geom.cx[y] - geom.rest_u[0][2])).abs() < 1e-9);
        assert!(geom.du[li][h].abs() < 3.0);
        assert!(geom.du[lm][y].abs() < 3.5);

        // Same-hand travel still uses the mirrored hand-relative coordinate.
        assert!((geom.du[ri][h] - (geom.u[h] - geom.rest_u[1][3])).abs() < 1e-9);
    }

    #[test]
    fn cross_reach_rejects_far_cross_body_keys() {
        let geom = Geometry::new();
        let q = key_index("Q").unwrap();
        let rp = parse_finger8("RP").unwrap();
        assert!(!geom.cross_reach[q][rp]);
        assert!(!geom.is_reachable(q, rp));
    }
}
