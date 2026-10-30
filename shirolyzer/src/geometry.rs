//! ANSI physical geometry: stagger, hand, row, rest posture, reach, and the
//! key/finger displacement tables used by the comfort model.
//!
//! This is a direct port of `shirolyzer/utils/ansi/ansi.py` and
//! `shirolyzer/utils/fingers.py`.  The Python code uses a Delaunay
//! triangulation only as an inside-the-approved-hull test; an explicit convex
//! hull gives the same answer.

pub const ROWS: [&str; 3] = ["QWERTYUIOP", "ASDFGHJKL;", "ZXCVBNM,./"];
pub const ROW_OFFSET: [f64; 3] = [0.0, 0.25, 0.75];
pub const FINGERS: [char; 4] = ['P', 'R', 'M', 'I'];
pub const NK: usize = 30;
pub const NF: usize = 4;

pub const KEYS: [&str; NK] = [
    "Q", "W", "E", "R", "T", "Y", "U", "I", "O", "P", "A", "S", "D", "F", "G", "H", "J", "K", "L",
    ";", "Z", "X", "C", "V", "B", "N", "M", ",", ".", "/",
];

/// Finger length rank.  Python: `np.array([0, 1, 3, 2])` for P/R/M/I.
pub const LENGTH_RANK: [f64; NF] = [0.0, 1.0, 3.0, 2.0];

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

pub fn hand_of(key: &str) -> usize {
    match key_index(key) {
        Some(i) => Hand::default_hand()[i],
        None => 0,
    }
}

pub struct Geometry {
    pub cx: [f64; NK],
    pub cy: [f64; NK],
    pub hand: [usize; NK],
    pub row: [usize; NK],
    pub u: [f64; NK],
    pub v: [f64; NK],
    pub rest_u: [[f64; NF]; 2],
    pub rest_v: [[f64; NF]; 2],
    pub inner_sep: [[f64; NF]; 2],
    pub du: [[f64; NK]; NF],
    pub dv: [[f64; NK]; NF],
    pub reach: [[bool; NF]; NK],
    pub unlisted: [[bool; NF]; NK],
}

impl Default for Geometry {
    fn default() -> Self {
        Self::new()
    }
}

impl Geometry {
    pub fn new() -> Self {
        let mut cx = [0.0; NK];
        let mut cy = [0.0; NK];
        let mut hand = [0usize; NK];
        let mut row = [0usize; NK];
        let mut idx = 0;
        for (r, row_name) in ROWS.iter().enumerate() {
            for (c, _ch) in row_name.chars().enumerate() {
                cx[idx] = c as f64 + ROW_OFFSET[r] + 0.5;
                cy[idx] = r as f64 + 0.5;
                hand[idx] = if c < 5 { 0 } else { 1 };
                row[idx] = r;
                idx += 1;
            }
        }
        let mut u = [0.0; NK];
        let mut v = [0.0; NK];
        for i in 0..NK {
            u[i] = if hand[i] == 0 { cx[i] } else { -cx[i] };
            v[i] = cy[i];
        }

        const REST: [[(usize, char); NF]; 2] = [
            [(10, 'P'), (1, 'R'), (2, 'M'), (13, 'I')],
            [(19, 'P'), (8, 'R'), (7, 'M'), (16, 'I')],
        ];
        let mut rest_u = [[0.0; NF]; 2];
        let mut rest_v = [[0.0; NF]; 2];
        for h in 0..2 {
            for f in 0..NF {
                let (k, _) = REST[h][f];
                rest_u[h][f] = u[k];
                rest_v[h][f] = v[k];
            }
        }
        let mut inner_sep = [[0.0; NF]; 2];
        for h in 0..2 {
            for f in 0..NF - 1 {
                inner_sep[h][f] = rest_u[h][f + 1] - rest_u[h][f];
            }
            inner_sep[h][NF - 1] = 99.0;
        }

        let mut du = [[0.0; NK]; NF];
        let mut dv = [[0.0; NK]; NF];
        for k in 0..NK {
            for f in 0..NF {
                du[f][k] = u[k] - rest_u[hand[k]][f];
                dv[f][k] = v[k] - rest_v[hand[k]][f];
            }
        }

        let approved = approved_points();
        let mut reach = [[false; NF]; NK];
        for (f, points) in approved.iter().enumerate() {
            let hull = convex_hull(points);
            for k in 0..NK {
                reach[k][f] = point_in_hull(&hull, du[f][k], dv[f][k], 1e-9);
            }
        }
        let listed = listed_keys();
        let mut unlisted = [[false; NF]; NK];
        for k in 0..NK {
            for f in 0..NF {
                if reach[k][f] && !listed[k][f] {
                    unlisted[k][f] = true;
                }
            }
        }
        Self {
            cx,
            cy,
            hand,
            row,
            u,
            v,
            rest_u,
            rest_v,
            inner_sep,
            du,
            dv,
            reach,
            unlisted,
        }
    }

    pub fn same_hand(&self, a: usize, b: usize) -> bool {
        self.hand[a] == self.hand[b]
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

fn approved_points() -> [Vec<(f64, f64)>; NF] {
    let p = vec![(-0.5, 1.0), (-0.25, -1.0), (0.0, 0.0), (0.5, 1.0)];
    let r = vec![
        (-1.25, 1.0),
        (-1.0, 0.0),
        (-0.75, 2.0),
        (-0.25, 1.0),
        (0.0, 0.0),
        (0.25, 1.0),
        (0.25, 2.0),
        (0.75, 1.0),
        (0.75, 2.0),
        (1.0, 0.0),
        (2.0, 0.0),
    ];
    let m = vec![
        (-2.0, 0.0),
        (-1.75, 2.0),
        (-1.25, 1.0),
        (-1.0, 0.0),
        (-0.75, 1.0),
        (-0.75, 2.0),
        (-0.25, 1.0),
        (-0.25, 2.0),
        (0.0, 0.0),
        (0.25, 1.0),
        (0.25, 2.0),
        (0.75, 1.0),
        (0.75, 2.0),
        (1.0, 0.0),
        (1.25, 1.0),
        (1.75, 2.0),
        (2.0, 0.0),
        (2.25, 1.0),
        (2.75, 2.0),
    ];
    let i = vec![
        (-2.5, 1.0),
        (-2.0, 0.0),
        (-1.5, 1.0),
        (-1.0, 0.0),
        (-0.5, 1.0),
        (-0.25, -1.0),
        (0.0, 0.0),
        (0.25, -1.0),
        (0.5, 1.0),
        (0.75, -1.0),
        (1.0, 0.0),
        (1.25, -1.0),
        (1.5, 1.0),
    ];
    [p, r, m, i]
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

fn listed_keys() -> [[bool; NF]; NK] {
    let mut out = [[false; NF]; NK];
    for k in 0..NK {
        for f in 0..NF {
            let listed = match (Hand::default_hand()[k], FINGERS[f]) {
                (0, 'P') => "QAZ",
                (1, 'P') => ";./",
                (0, 'R') => "QWERSXA",
                (1, 'R') => "UIOPKL;,.",
                (0, 'M') => "QWERTSDFGXCVB",
                (1, 'M') => "YUIOJKLM,.",
                (0, 'I') => "WXRTSDFGCVB",
                (1, 'I') => "YUHJKLNM,.",
                _ => "",
            };
            out[k][f] = listed.contains(KEYS[k]);
        }
    }
    out
}
