//! Pattern comfort model: parameters, key costs, pair costs, and trigram costs.
//!
//! Port of `shirolyzer/utils/comfort.py`.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{anyhow, Result};
use serde_json::Value;

use crate::geometry::{
    finger_hand, finger_kind, finger_token, length_rank, Geometry, FINGERS, NF, NFINGER, NK,
};
use crate::redirect::{redirect_cost, RedirectCostTable};
use crate::roll::RollCreditTable;
use crate::scissor::ScissorTierTable;

pub const PARAM_NAMES: [&str; 47] = [
    "e_dn",
    "e_up_I",
    "e_up_P",
    "e_lat",
    "e_lat2",
    "e_unl",
    "e_dn2",
    "e_ov",
    "p",
    "a_MI",
    "a_RM",
    "a_PR",
    "a_RI",
    "a_PM",
    "a_PI",
    "b_MI",
    "c_MIrev",
    "m_PR",
    "m_RM",
    "m_MI",
    "m_skip",
    "kappa_MI",
    "kappa_o",
    "varsig",
    "lam_x",
    "lam_prox",
    "lam_spread",
    "w_spread",
    "r_MI",
    "r_RM_in",
    "r_RM_out",
    "r_PR_out",
    "r_PR_in",
    "leap2",
    "leap3",
    "leap_P",
    "gamma13",
    "e_nd",
    "nd_waive",
    "shift_thr",
    "shift_disc",
    "e_ndgap",
    "e_nd_rest",
    "scis_lat",
    "scis_dir",
    "e_upcol",
    "gamma13_alt",
];

pub const PARAM_DEFAULTS: [f64; 47] = [
    0.14, 0.15, 1.0, 0.20, 0.10, 0.60, 0.20, 0.30, 2.0, 0.15, 1.50, 1.50, 1.00, 0.50, 0.50, 0.20,
    3.00, 0.50, 0.25, 0.25, 0.75, 0.40, 0.00, 0.35, 0.30, 0.50, 1.00, 1.00, -0.275, -0.45, -0.20,
    -0.05, -0.40, 0.15, 0.35, 0.15, 0.50, 0.30, 1.00, 0.60, 0.30, 0.50, 0.30, 0.50, 0.30, 0.30,
    0.80,
];

// Declared constants (not fitted).
pub const B_IMP: f64 = 9.0;
pub const G_IMP: f64 = 1.5;
pub const G_IMP_RP: f64 = 1.5;
pub const DELTA_IMP: f64 = 0.30;
pub const SPREAD_LIM: f64 = 2.0;
pub const SPREAD_IMP: f64 = 6.0;
pub const IP_SMIN: f64 = 1.9;
pub const TAU: f64 = 0.05;
pub const W_ALT: f64 = -0.12;
pub const PAIR_CAP: f64 = 12.0;
pub const W_REPEAT: f64 = 0.6;
pub const W_REPEAT_SWAP: f64 = 0.5;
pub const W_REDIRECT: f64 = 0.30;
pub const W_REDIRECT_WEAK: f64 = 1.80;
pub const W_CONTINUE: f64 = -0.10;
pub const W_SFS_1U: f64 = 2.50;
pub const W_SFS_FAR: f64 = 3.50;
/// Roll credit of tier 1 in `roll_tiers`; later tiers earn equal smaller shares.
pub const W_ROLL_TIER_BEST: f64 = -0.60;
pub const ROLL_D0: f64 = 1.25;
pub const ROLL_D1: f64 = 1.0;

#[inline]
pub fn e_idx(k: usize, f: usize) -> usize {
    k * NF + f
}

#[inline]
pub fn pt_idx(a: usize, fa: usize, b: usize, fb: usize) -> usize {
    ((a * NF + fa) * NK + b) * NF + fb
}

#[inline]
pub fn sig(x: f64) -> f64 {
    1.0 / (1.0 + (-x.clamp(-50.0, 50.0)).exp())
}

pub fn sfb_cost(geom: &Geometry, a: usize, b: usize) -> f64 {
    let dr = (geom.v[a] - geom.v[b]).abs();
    let mut c = 5.0 + 0.5 * dr + 0.5 * dr * dr;
    if geom.v[b] > geom.v[a] && dr >= 1.0 {
        c += 0.3;
    }
    c
}

#[derive(Clone, Copy, Debug)]
pub struct PairFeature {
    pub same_hand: bool,
    pub diff_f: bool,
    pub g: f64,
    pub s: f64,
    pub drow: f64,
    pub dist: f64,
    pub lo_f: usize,
    pub hi_f: usize,
    pub nadj: usize,
    pub direction: f64,
    pub rest_sep: f64,
    pub second_longer: bool,
    pub a_longer: bool,
}

pub fn pair_feature(geom: &Geometry, a: usize, fa: usize, b: usize, fb: usize) -> PairFeature {
    let ka = finger_kind(fa);
    let kb = finger_kind(fb);
    let a_longer = length_rank(fa) > length_rank(fb);
    let g = if a_longer {
        geom.dv[fa][a] - geom.dv[fb][b]
    } else {
        geom.dv[fb][b] - geom.dv[fa][a]
    };
    let inner_is_a = ka > kb;
    let s = if inner_is_a {
        geom.u8[fa][a] - geom.u8[fb][b]
    } else {
        geom.u8[fb][b] - geom.u8[fa][a]
    };
    let lo_f = ka.min(kb);
    let hi_f = ka.max(kb);
    let dist = ((geom.cx[a] - geom.cx[b]).powi(2) + (geom.v[a] - geom.v[b]).powi(2)).sqrt();
    let finger_hand_a = finger_hand(fa);
    PairFeature {
        same_hand: finger_hand_a == finger_hand(fb),
        diff_f: fa != fb,
        g,
        s,
        drow: (geom.v[a] - geom.v[b]).abs(),
        dist,
        lo_f,
        hi_f,
        nadj: hi_f - lo_f,
        direction: (kb as f64 - ka as f64).signum(),
        rest_sep: geom.rest_u[finger_hand_a][hi_f] - geom.rest_u[finger_hand_a][lo_f],
        second_longer: length_rank(fb) > length_rank(fa),
        a_longer,
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Scales {
    pub sfb: f64,
    pub scissor: f64,
    pub redirect: f64,
    pub roll: f64,
    pub alt: f64,
    pub redirect_costs: Option<RedirectCostTable>,
    pub roll_credits: Option<RollCreditTable>,
    pub scissor_tiers: Option<ScissorTierTable>,
}

impl Default for Scales {
    fn default() -> Self {
        Self {
            sfb: 1.0,
            scissor: 1.0,
            redirect: 1.0,
            roll: 1.0,
            alt: 1.0,
            redirect_costs: None,
            roll_credits: None,
            scissor_tiers: None,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Parts {
    pub vert: Vec<f64>,
    pub cross: Vec<f64>,
    pub spread: Vec<f64>,
    pub roll: Vec<f64>,
    pub leap: Vec<f64>,
    pub shift: Vec<f64>,
}

#[derive(Clone, Debug)]
pub struct ModelTables {
    pub e: Vec<f64>,
    pub pt: Vec<f64>,
    pub t2: Vec<f64>,
    pub strain: Vec<f64>,
    pub parts: Parts,
    /// Severity band for each directed finger pair, or `u8::MAX` when the pair
    /// is not a scissor.
    pub scissor_band: Vec<u8>,
}

pub struct Model {
    pub params: Vec<f64>,
    index: HashMap<&'static str, usize>,
    pub disallow_full_reverse: [[bool; NFINGER]; NFINGER],
    pub default_finger: [usize; NK],
    pub finger_strength: [f64; NF],
    pub scales: Scales,
}

impl Model {
    pub fn new(
        parameters: Vec<f64>,
        disallow_full_reverse: [[bool; NFINGER]; NFINGER],
        default_finger: [usize; NK],
    ) -> Self {
        let mut index = HashMap::new();
        for (i, name) in PARAM_NAMES.iter().enumerate() {
            index.insert(*name, i);
        }
        Self {
            params: parameters,
            index,
            disallow_full_reverse,
            default_finger,
            finger_strength: [0.0; NF],
            scales: Scales::default(),
        }
    }

    pub fn from_file(
        path: &Path,
        disallow_full_reverse: [[bool; NFINGER]; NFINGER],
        default_finger: [usize; NK],
    ) -> Result<Self> {
        let data: Value = serde_json::from_str(&crate::paths::read_to_string(path)?)?;
        let obj = data
            .as_object()
            .ok_or_else(|| anyhow!("{}: parameters must be a JSON object", path.display()))?;
        let mut values = PARAM_DEFAULTS.to_vec();
        for (i, name) in PARAM_NAMES.iter().enumerate() {
            if let Some(v) = obj.get(*name) {
                values[i] = v.as_f64().ok_or_else(|| {
                    anyhow!("{}: parameter {name} is not a number", path.display())
                })?;
            }
        }
        Ok(Self::new(values, disallow_full_reverse, default_finger))
    }

    #[inline]
    pub fn p(&self, name: &str) -> f64 {
        self.params[self.index[name]]
    }

    pub fn default_finger_index(&self) -> [usize; NK] {
        self.default_finger
    }

    pub fn key_cost(&self, geom: &Geometry) -> (Vec<f64>, Vec<f64>) {
        self.key_cost_impl(geom, false)
    }

    fn key_cost_impl(&self, geom: &Geometry, ignore_reach: bool) -> (Vec<f64>, Vec<f64>) {
        let mut e = vec![0.0; NK * NF];
        let mut nd = vec![0.0; NK * NF];
        let mut ndr = vec![0.0; NK * NF];
        let dflt = self.default_finger_index();
        let mut is_rest = [false; NK];
        for keys in &geom.rest_key {
            for &k in keys {
                is_rest[k] = true;
            }
        }
        let up_pen = [self.p("e_up_P"), 0.0, 0.0, self.p("e_up_I")];
        for k in 0..NK {
            for f in 0..NF {
                let kind = finger_kind(f);
                let hand = finger_hand(f);
                let dv = geom.dv[f][k];
                let du = geom.du[f][k];
                let colup = if kind == 3 {
                    self.p("e_upcol")
                        * (1.0 - du.abs() / 0.5).max(0.0)
                        * if dv < 0.0 { 1.0 } else { 0.0 }
                } else {
                    0.0
                };
                let ov = if kind < 3 {
                    (du - geom.inner_sep[hand][kind]).max(0.0)
                } else {
                    0.0
                };
                let mut cost = colup
                    + self.p("e_dn") * dv.max(0.0)
                    + up_pen[kind] * (-dv).max(0.0)
                    + self.p("e_lat") * du.abs()
                    + self.p("e_lat2") * (du.abs() - 1.0).max(0.0).powi(2)
                    + self.p("e_dn2") * (dv - 1.0).max(0.0).powi(2)
                    + self.p("e_ov") * ov;
                cost += self.p("e_unl") * if geom.unlisted[k][f] { 1.0 } else { 0.0 };
                e[e_idx(k, f)] = cost;
                if !is_rest[k] {
                    nd[e_idx(k, f)] = if f == dflt[k] { 0.0 } else { 1.0 };
                } else {
                    ndr[e_idx(k, f)] = if f == dflt[k] { 0.0 } else { 1.0 };
                }
            }
        }
        let e_nd = self.p("e_nd");
        let e_ndgap = self.p("e_ndgap");
        let e_nd_rest = self.p("e_nd_rest");
        let mut final_e = vec![f64::INFINITY; NK * NF];
        let mut final_nd = vec![0.0; NK * NF];
        for k in 0..NK {
            let def = e[e_idx(k, dflt[k])];
            for f in 0..NF {
                let i = e_idx(k, f);
                let gap = if def.is_finite() && e[i].is_finite() && (nd[i] + ndr[i]) > 0.0 {
                    (def - e[i]).max(0.0)
                } else {
                    0.0
                };
                let ndv = e_nd * nd[i] + e_ndgap * gap;
                final_nd[i] = ndv;
                let mut value = e[i] + ndv + e_nd_rest * ndr[i] + self.finger_strength[f];
                if !ignore_reach && !geom.is_reachable(k, f) {
                    value = f64::INFINITY;
                }
                final_e[i] = value;
            }
        }
        (final_e, final_nd)
    }

    fn alpha(&self, lo: usize, hi: usize) -> f64 {
        match (lo, hi) {
            (0, 1) => self.p("a_PR"),
            (1, 2) => self.p("a_RM"),
            (2, 3) => self.p("a_MI"),
            (0, 2) => self.p("a_PM"),
            (1, 3) => self.p("a_RI"),
            (0, 3) => self.p("a_PI"),
            _ => 0.0,
        }
    }

    /// Direction bias for a same-hand adjacent roll.
    ///
    /// The bias grows with the weaker finger in the pair: index/middle has no
    /// directional preference, middle/ring a small ring-first preference, and
    /// ring/pinky a strong pinky-first preference.  `roll_tiers` may override
    /// this for the directed pairs it names.
    fn roll_direction_bias(&self, lo: usize, hi: usize, direction: f64) -> f64 {
        match (lo, hi) {
            (2, 3) => self.p("r_MI"),
            (1, 2) if direction > 0.0 => self.p("r_RM_in"),
            (1, 2) => self.p("r_RM_out"),
            (0, 1) if direction > 0.0 => self.p("r_PR_in"),
            (0, 1) => self.p("r_PR_out"),
            _ => 0.0,
        }
    }

    pub fn pair_parts(&self, geom: &Geometry, nd: &[f64]) -> (Vec<f64>, Parts) {
        let size = NK * NF * NK * NF;
        let mut parts = Parts {
            vert: vec![0.0; size],
            cross: vec![0.0; size],
            spread: vec![0.0; size],
            roll: vec![0.0; size],
            leap: vec![0.0; size],
            shift: vec![0.0; size],
        };
        let dflt = self.default_finger_index();
        let p = self.p("p");
        for a in 0..NK {
            for fa in 0..NF {
                for b in 0..NK {
                    for fb in 0..NF {
                        let pf = pair_feature(geom, a, fa, b, fb);
                        if !(pf.same_hand && pf.diff_f) {
                            continue;
                        }
                        let i = pt_idx(a, fa, b, fb);
                        let g = pf.g;
                        let s = pf.s;
                        let drow = pf.drow;
                        let lo = pf.lo_f;
                        let hi = pf.hi_f;
                        let nadj = pf.nadj;
                        let alpha = self.alpha(lo, hi);
                        let beta = if lo == 2 && hi == 3 {
                            self.p("b_MI")
                        } else {
                            0.0
                        };
                        let adj_mr_rp = nadj == 1 && lo <= 1;
                        let dirf = 1.0
                            + if adj_mr_rp { self.p("scis_dir") } else { 0.0 }
                                * if pf.second_longer { 1.0 } else { -1.0 };
                        let latf = 1.0
                            + if adj_mr_rp { self.p("scis_lat") } else { 0.0 }
                                * 0.5
                                * (geom.du[fa][a].abs() + geom.du[fb][b].abs());
                        let mut vert = if g > 0.0 {
                            alpha * g.abs().powf(p) * dirf * latf
                        } else {
                            beta * g.abs().powf(p)
                        };
                        let thr = if lo == 0 { G_IMP_RP } else { G_IMP };
                        let step = if self.disallow_full_reverse[lo][hi] || lo == 0 {
                            B_IMP * sig((g - thr) / TAU)
                        } else {
                            5.85 * sig((g - thr) / TAU)
                        };
                        if adj_mr_rp {
                            vert += step;
                        }
                        if lo == 2 && hi == 3 {
                            vert += self.p("c_MIrev") * sig((-g - 0.5) / TAU);
                        }

                        let m = [self.p("m_PR"), self.p("m_RM"), self.p("m_MI")];
                        let mut cover = 0.0;
                        for (gap, value) in m.iter().enumerate() {
                            if lo <= gap && hi > gap {
                                cover += *value;
                            }
                        }
                        cover += self.p("m_skip") * (nadj as f64 - 1.0).max(0.0);
                        let kap = if lo == 2 && hi == 3 {
                            self.p("kappa_MI")
                        } else {
                            self.p("kappa_o")
                        };
                        let need = cover - kap * drow.min(2.0) + self.p("varsig") * g.max(0.0);
                        let deficit = need - s;
                        let prox = if nadj == 1 {
                            (0.8 * pf.rest_sep - s).max(0.0)
                        } else {
                            0.0
                        };
                        let ipstep = if lo == 0 && hi == 3 {
                            B_IMP * sig((IP_SMIN - s) / TAU)
                        } else {
                            0.0
                        };
                        let cross = B_IMP * sig((deficit - DELTA_IMP) / TAU)
                            + ipstep
                            + self.p("lam_x") * (-s).max(0.0)
                            + self.p("lam_prox") * prox;

                        let excess = s - (pf.rest_sep + self.p("w_spread"));
                        let mut spread = self.p("lam_spread") * excess.max(0.0);
                        if nadj == 1 {
                            spread += SPREAD_IMP * sig((s - pf.rest_sep - SPREAD_LIM) / TAU);
                        }

                        let mut rw = self.roll_direction_bias(lo, hi, pf.direction);
                        if nadj == 1 {
                            // roll_tiers overrides the model's weight for the
                            // directed pairs it names; unlisted pairs keep it.
                            if let Some(credit) = self
                                .scales
                                .roll_credits
                                .as_ref()
                                .and_then(|table| table.credit_for_fingers([fa, fb]))
                            {
                                rw = credit;
                            }
                        }
                        let fade = (1.0 - (pf.dist - ROLL_D0).max(0.0) / ROLL_D1).clamp(0.0, 1.0);
                        let supp = if deficit > 0.2 { 0.25 } else { 1.0 };
                        let roll = if nadj == 1 { rw * fade * supp } else { 0.0 };
                        let mut leap = 0.0;
                        if nadj == 2 {
                            leap += self.p("leap2");
                        }
                        if nadj == 3 {
                            leap += self.p("leap3");
                        }
                        if nadj >= 2 && lo == 0 {
                            leap += self.p("leap_P");
                        }

                        let nd_b = nd[e_idx(b, fb)];
                        let nd_a = nd[e_idx(a, fa)];
                        let waive = -self.p("nd_waive")
                            * (nd_b * if fa == dflt[b] { 1.0 } else { 0.0 }
                                + nd_a * if fb == dflt[a] { 1.0 } else { 0.0 })
                            * if (vert + cross) < 4.0 { 1.0 } else { 0.0 };
                        let dua = geom.du[fa][a];
                        let dub = geom.du[fb][b];
                        let shared = if dua.signum() * dub.signum() > 0.0 {
                            dua.abs().min(dub.abs())
                        } else {
                            0.0
                        };
                        let shift = -self.p("shift_disc")
                            * self.p("e_lat")
                            * (shared - self.p("shift_thr")).max(0.0);

                        parts.vert[i] = vert * self.scales.scissor;
                        parts.cross[i] = cross * self.scales.scissor;
                        parts.spread[i] = spread;
                        parts.roll[i] = roll * self.scales.roll;
                        parts.leap[i] = leap * self.scales.roll;
                        parts.shift[i] = shift + waive;
                    }
                }
            }
        }
        let mut terrain = vec![0.0; size];
        for i in 0..size {
            terrain[i] = parts.vert[i] + parts.cross[i] + parts.spread[i];
        }
        (terrain, parts)
    }

    pub fn tables(&self, geom: &Geometry) -> ModelTables {
        self.tables_impl(geom, false)
    }

    /// Same as [`Self::tables`], but ignores geometry reachability.  Used only
    /// for explicit alt rules, which are allowed to name otherwise unreachable
    /// fingers.
    pub fn tables_unrestricted(&self, geom: &Geometry) -> ModelTables {
        self.tables_impl(geom, true)
    }

    fn tables_impl(&self, geom: &Geometry, ignore_reach: bool) -> ModelTables {
        let (e, nd) = self.key_cost_impl(geom, ignore_reach);
        let (mut strain, mut parts) = self.pair_parts(geom, &nd);
        let size = NK * NF * NK * NF;
        let mut scissor_band = vec![u8::MAX; size];
        let mut pt = vec![0.0; size];
        let mut t2 = vec![0.0; size];
        for a in 0..NK {
            for fa in 0..NF {
                for b in 0..NK {
                    for fb in 0..NF {
                        let i = pt_idx(a, fa, b, fb);
                        let pf = pair_feature(geom, a, fa, b, fb);
                        let mut tot = parts.vert[i]
                            + parts.cross[i]
                            + parts.spread[i]
                            + parts.roll[i]
                            + parts.leap[i]
                            + parts.shift[i];
                        let scissor_like = pf.same_hand && pf.diff_f && pf.g > 0.25;
                        if scissor_like {
                            let base_cost = e[e_idx(a, fa)]
                                + e[e_idx(b, fb)]
                                + tot
                                + crate::scissor::SCISSOR_STRAIN_OFFSET;
                            let band = crate::scissor::band(base_cost);
                            let mult = self
                                .scales
                                .scissor_tiers
                                .as_ref()
                                .map(|table| table.multiplier_for_band(band))
                                .unwrap_or(1.0);
                            scissor_band[i] = band as u8;
                            parts.vert[i] *= mult;
                            parts.cross[i] *= mult;
                            parts.spread[i] *= mult;
                            strain[i] *= mult;
                            tot = parts.vert[i]
                                + parts.cross[i]
                                + parts.spread[i]
                                + parts.roll[i]
                                + parts.leap[i]
                                + parts.shift[i];
                        }
                        if tot > PAIR_CAP {
                            tot = PAIR_CAP;
                        }
                        let mut out = if pf.same_hand {
                            tot
                        } else {
                            W_ALT * self.scales.alt
                        };
                        let same_f = pf.same_hand && fa == fb && a != b;
                        if same_f {
                            out = sfb_cost(geom, a, b) * self.scales.sfb;
                        }
                        if a == b {
                            out = W_REPEAT + if fa != fb { W_REPEAT_SWAP } else { 0.0 };
                        }
                        let reach =
                            ignore_reach || (geom.is_reachable(a, fa) && geom.is_reachable(b, fb));
                        if !reach {
                            out = f64::INFINITY;
                        }
                        pt[i] = out;
                        t2[i] = e[e_idx(a, fa)] + e[e_idx(b, fb)] + out;
                    }
                }
            }
        }
        ModelTables {
            e,
            pt,
            t2,
            strain,
            parts,
            scissor_band,
        }
    }

    pub fn bi_many(&self, tabs: &ModelTables, a: &[usize], b: &[usize]) -> Vec<f64> {
        let n = a.len();
        let mut out = vec![0.0; n * NF * NF];
        for i in 0..n {
            for fa in 0..NF {
                for fb in 0..NF {
                    let oi = (i * NF + fa) * NF + fb;
                    if a[i] == b[i] && fa != fb {
                        out[oi] = f64::INFINITY;
                    } else {
                        out[oi] = tabs.t2[pt_idx(a[i], fa, b[i], fb)];
                    }
                }
            }
        }
        out
    }

    pub fn tri_many(
        &self,
        geom: &Geometry,
        tabs: &ModelTables,
        a: &[usize],
        b: &[usize],
        c: &[usize],
    ) -> Vec<f64> {
        let mut out = vec![0.0; a.len() * NF * NF * NF];
        self.tri_many_into(geom, tabs, a, b, c, &mut out);
        out
    }

    pub fn tri_many_into(
        &self,
        geom: &Geometry,
        tabs: &ModelTables,
        a: &[usize],
        b: &[usize],
        c: &[usize],
        out: &mut [f64],
    ) {
        let n = a.len();
        debug_assert_eq!(out.len(), n * NF * NF * NF);
        let gamma13 = self.p("gamma13");
        let gamma13_alt = self.p("gamma13_alt");
        for i in 0..n {
            let ka = a[i];
            let kb = b[i];
            let kc = c[i];
            let d_ac = geom.distance(ka, kc);
            let sfs = if d_ac <= 1.3 { W_SFS_1U } else { W_SFS_FAR } * self.scales.sfb;
            for f1 in 0..NF {
                for f2 in 0..NF {
                    for f3 in 0..NF {
                        let d1 = f2 as i32 - f1 as i32;
                        let d2 = f3 as i32 - f2 as i32;
                        let turn = d1 * d2 < 0;
                        let cont = d1 * d2 > 0;
                        let mut cost = tabs.e[e_idx(ka, f1)]
                            + tabs.e[e_idx(kb, f2)]
                            + tabs.e[e_idx(kc, f3)]
                            + tabs.pt[pt_idx(ka, f1, kb, f2)]
                            + tabs.pt[pt_idx(kb, f2, kc, f3)];
                        let same = finger_hand(f1) == finger_hand(f2)
                            && finger_hand(f2) == finger_hand(f3);
                        if same {
                            let redirect = if turn {
                                redirect_cost(self.scales.redirect_costs.as_ref(), [f1, f2, f3])
                            } else {
                                0.0
                            };
                            let add = (redirect + if cont { W_CONTINUE } else { 0.0 })
                                * self.scales.redirect;
                            cost += add;
                            let s13 = tabs.strain[pt_idx(ka, f1, kc, f3)];
                            let s13 = if s13.is_finite() { s13 } else { 0.0 };
                            if f1 != f3 && ka != kc {
                                cost += gamma13 * self.scales.scissor * s13;
                            }
                        } else {
                            let s13 = tabs.strain[pt_idx(ka, f1, kc, f3)];
                            let s13 = if s13.is_finite() { s13 } else { 0.0 };
                            let alt13 = finger_hand(f1) == finger_hand(f3)
                                && finger_hand(f1) != finger_hand(f2)
                                && ka != kc;
                            if alt13 && f1 != f3 {
                                cost += gamma13_alt * self.scales.scissor * s13;
                            }
                        }
                        let f1_eq_f2 = f1 == f2;
                        let m13 = f1 == f3
                            && !(finger_hand(f1) == finger_hand(f2) && f1_eq_f2)
                            && finger_hand(f1) == finger_hand(f3)
                            && ka != kc;
                        if m13 {
                            cost += sfs;
                        }
                        let bad = (ka == kb && f1 != f2)
                            || (kb == kc && f2 != f3)
                            || (ka == kc && f1 != f3);
                        let oi = ((i * NF + f1) * NF + f2) * NF + f3;
                        out[oi] = if bad { f64::INFINITY } else { cost };
                    }
                }
            }
        }
    }

    pub fn tri_block(
        &self,
        geom: &Geometry,
        tabs: &ModelTables,
        a: usize,
        b: usize,
        c: usize,
    ) -> Vec<f64> {
        self.tri_many(geom, tabs, &[a], &[b], &[c])
    }
}

pub fn finger_letter(index: usize) -> char {
    FINGERS[finger_kind(index)]
}

pub fn finger_name(index: usize) -> &'static str {
    finger_token(index)
}

pub fn parse_finger(token: &str) -> Option<usize> {
    crate::geometry::parse_finger8(token)
        .or_else(|| token.chars().last().and_then(crate::geometry::finger_index))
}

/// Validate that every parameter name exists in a JSON object.
pub fn missing_parameters(data: &Value) -> Vec<String> {
    let Some(obj) = data.as_object() else {
        return PARAM_NAMES.iter().map(|s| s.to_string()).collect();
    };
    PARAM_NAMES
        .iter()
        .filter(|name| !obj.contains_key(**name))
        .map(|s| s.to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::{key_index, parse_finger8, Geometry};

    #[test]
    fn rest_keys_follow_geometry_instead_of_a_hardcoded_list() {
        let geom = Geometry::new();
        let cfg = crate::config::load(None).unwrap();
        let model = crate::config::build_model(&cfg).unwrap();
        let q = key_index("Q").unwrap();
        let lp = parse_finger8("LP").unwrap();
        let (before, _) = model.key_cost(&geom);
        let mut moved = geom.clone();
        moved.rest_key = [[q; NFINGER]; 2];
        let (after, _) = model.key_cost(&moved);
        let expected = model.p("e_nd") - model.p("e_nd_rest");
        assert!(
            (before[e_idx(q, lp)] - after[e_idx(q, lp)] - expected).abs() < 1e-12,
            "before={} after={} expected={}",
            before[e_idx(q, lp)],
            after[e_idx(q, lp)],
            expected
        );
    }

    #[test]
    fn finger_strength_adds_a_constant_to_each_finger_state() {
        let geom = Geometry::new();
        let cfg = crate::config::load(None).unwrap();
        let mut model = crate::config::build_model(&cfg).unwrap();
        let (before, _) = model.key_cost(&geom);
        let lp = parse_finger8("LP").unwrap();
        model.finger_strength[lp] = 0.5;
        let (after, _) = model.key_cost(&geom);
        let a = key_index("A").unwrap();
        assert!((after[e_idx(a, lp)] - before[e_idx(a, lp)] - 0.5).abs() < 1e-12);
    }

    #[test]
    fn cross_hand_key_costs_are_finite_and_not_hand_relative() {
        let geom = Geometry::new();
        let cfg = crate::config::load(None).unwrap();
        let model = crate::config::build_model(&cfg).unwrap();
        let (e, _) = model.key_cost(&geom);
        let h = key_index("H").unwrap();
        let y = key_index("Y").unwrap();
        let li = parse_finger8("LI").unwrap();
        let lm = parse_finger8("LM").unwrap();
        let ri = parse_finger8("RI").unwrap();

        assert!(e[e_idx(h, li)].is_finite());
        assert!(e[e_idx(y, lm)].is_finite());
        assert!(e[e_idx(h, ri)].is_finite());
        assert!(e[e_idx(h, li)] < 3.0);
        assert!(e[e_idx(y, lm)] < 3.5);
    }

    #[test]
    fn roll_tiers_credit_only_the_listed_directed_pairs() {
        use crate::roll::{pair_for, RollTiers};

        let geom = Geometry::new();
        let cfg = crate::config::load(None).unwrap();
        let mut model = crate::config::build_model(&cfg).unwrap();
        model.scales.roll_credits = None;
        let base = model.tables(&geom);
        model.scales.roll_credits = Some(RollTiers::new(vec![vec![[1, 2]]]).credit_table());
        let tiered = model.tables(&geom);

        let mut changed = 0;
        for a in 0..NK {
            for fa in 0..NF {
                for b in 0..NK {
                    for fb in 0..NF {
                        let i = pt_idx(a, fa, b, fb);
                        let before = base.parts.roll[i];
                        let after = tiered.parts.roll[i];
                        if pair_for([fa, fb]) == Some([1, 2]) {
                            // index -> middle takes the tier credit, which is
                            // never weaker than the full -0.60 times the roll scale.
                            assert!(after <= 0.0);
                            assert!(
                                after.abs() <= W_ROLL_TIER_BEST.abs() * model.scales.roll + 1e-12
                            );
                            if (after - before).abs() > 1e-12 {
                                changed += 1;
                            }
                        } else {
                            assert_eq!(before, after, "unlisted pair changed: {a},{fa},{b},{fb}");
                        }
                    }
                }
            }
        }
        assert!(changed > 0, "the listed pair never changed");
    }

    #[test]
    fn redirect_tiers_price_only_configured_patterns() {
        let geom = Geometry::new();
        let cfg = crate::config::load(None).unwrap();
        let tiers = cfg.redirect_tiers.clone().expect("shipped redirect tiers");
        let mut model = crate::config::build_model(&cfg).unwrap();
        let configured_index = (NF + 2) * NF + 1;
        let unlisted_index = (3 * NF + 1) * NF + 2;
        let ring_middle_ring = [3, 2, 3];
        let unlisted = [1, 3, 2];

        let tables = model.tables(&geom);
        let mut chosen = None;
        'search: for a in 0..NK {
            for b in 0..NK {
                for c in 0..NK {
                    if a == b || b == c || a == c {
                        continue;
                    }
                    let block = model.tri_block(&geom, &tables, a, b, c);
                    if block[configured_index].is_finite() && block[unlisted_index].is_finite() {
                        chosen = Some((a, b, c, block));
                        break 'search;
                    }
                }
            }
        }
        let (a, b, c, block) = chosen.expect("reachable redirect test trigram");
        let configured = block[configured_index];
        let unlisted_cost = block[unlisted_index];

        model.scales.redirect_costs = None;
        let legacy_tables = model.tables(&geom);
        let legacy_block = model.tri_block(&geom, &legacy_tables, a, b, c);
        let legacy_configured = legacy_block[configured_index];
        let legacy_unlisted = legacy_block[unlisted_index];

        let expected =
            (crate::redirect::tier_cost(1, tiers.len()) - W_REDIRECT) * model.scales.redirect;
        assert!(
            (configured - legacy_configured - expected).abs() < 1e-12,
            "configured={configured} legacy={legacy_configured} expected={expected}"
        );
        assert!((unlisted_cost - legacy_unlisted).abs() < 1e-12);
        assert!(model.scales.redirect_costs.is_none());
        assert!(Scales::default().redirect_costs.is_none());
        assert_eq!(
            tiers.cost(ring_middle_ring),
            Some(crate::redirect::tier_cost(1, tiers.len()))
        );
        assert_eq!(tiers.cost(unlisted), None);
    }

    #[test]
    fn pair_feature_uses_the_fingers_hand() {
        let geom = Geometry::new();
        let h = key_index("H").unwrap();
        let y = key_index("Y").unwrap();
        let j = key_index("J").unwrap();
        let li = parse_finger8("LI").unwrap();
        let lm = parse_finger8("LM").unwrap();
        let ri = parse_finger8("RI").unwrap();

        // H and Y are right-hand keys, but LI/LM are both left fingers.
        let left_pair = pair_feature(&geom, h, li, y, lm);
        assert!(left_pair.same_hand);
        assert_eq!(left_pair.lo_f, 2);
        assert_eq!(left_pair.hi_f, 3);
        assert!((left_pair.rest_sep - (geom.rest_u[0][3] - geom.rest_u[0][2])).abs() < 1e-9);

        // A cross-hand fingering alternates hands even on adjacent right keys.
        let cross_pair = pair_feature(&geom, h, li, j, ri);
        assert!(!cross_pair.same_hand);

        // Same-hand pairs remain same-hand.
        assert!(pair_feature(&geom, h, ri, j, ri).same_hand);
    }
}
