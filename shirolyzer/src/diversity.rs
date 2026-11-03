//! Clustering of compute results by score tolerance and layout distance.

use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct Row {
    pub start: String,
    pub final_score: f64,
    pub primary: f64,
    pub secondary: Option<f64>,
    pub layout: String,
    pub reference_name: Option<String>,
}

impl Row {
    pub fn new(
        start: impl Into<String>,
        primary: f64,
        secondary: Option<f64>,
        layout: impl Into<String>,
    ) -> Self {
        Self {
            start: start.into(),
            final_score: secondary.unwrap_or(primary),
            primary,
            secondary,
            layout: layout.into(),
            reference_name: None,
        }
    }
}

pub fn layout_distance(a: &str, b: &str) -> usize {
    a.chars().zip(b.chars()).filter(|(x, y)| x != y).count()
}

/// Letter-only layout identity: punctuation placement does not change it.
pub fn letter_signature(layout: &str) -> String {
    layout.chars().filter(|c| c.is_ascii_alphabetic()).collect()
}

/// Count positions where the letter placement differs. Punctuation-only
/// differences are ignored.
pub fn letter_distance(a: &str, b: &str) -> usize {
    a.chars()
        .zip(b.chars())
        .filter(|(x, y)| x != y && (x.is_ascii_alphabetic() || y.is_ascii_alphabetic()))
        .count()
}

/// Pick the highest-ranked rows that differ by at least `min_letter_distance`
/// letter positions. If the list is too small, fill from remaining rows whose
/// letter signature is not already present.
pub fn select_diverse_layouts(rows: &[Row], count: usize, min_letter_distance: usize) -> Vec<Row> {
    let mut selected: Vec<Row> = Vec::new();
    for row in rows {
        if selected.len() >= count {
            break;
        }
        if selected
            .iter()
            .all(|kept| letter_distance(&row.layout, &kept.layout) >= min_letter_distance)
        {
            selected.push(row.clone());
        }
    }
    if selected.len() < count {
        for row in rows {
            if selected.len() >= count {
                break;
            }
            let signature = letter_signature(&row.layout);
            if !selected
                .iter()
                .any(|kept| letter_signature(&kept.layout) == signature)
            {
                selected.push(row.clone());
            }
        }
    }
    selected
}

/// Collapse duplicate layouts, keeping the best primary and secondary values.
pub fn merge_results(results: &[Row]) -> Vec<Row> {
    let mut by_layout: HashMap<String, Row> = HashMap::new();
    for row in results {
        match by_layout.get_mut(&row.layout) {
            None => {
                by_layout.insert(row.layout.clone(), row.clone());
            }
            Some(cur) => {
                if row.primary < cur.primary {
                    cur.primary = row.primary;
                    cur.start = row.start.clone();
                }
                if let Some(sec) = row.secondary {
                    if cur.secondary.is_none() || sec < cur.secondary.unwrap() {
                        cur.secondary = Some(sec);
                    }
                }
            }
        }
    }
    for row in by_layout.values_mut() {
        row.final_score = row.secondary.unwrap_or(row.primary);
    }
    by_layout.into_values().collect()
}

pub fn comparable(
    row: &Row,
    reference: &Row,
    primary_tolerance: f64,
    secondary_tolerance: f64,
) -> bool {
    if row.primary - reference.primary > primary_tolerance {
        return false;
    }
    if let Some(ref_sec) = reference.secondary {
        match row.secondary {
            None => return false,
            Some(sec) if sec - ref_sec > secondary_tolerance => return false,
            _ => {}
        }
    }
    true
}

pub fn select_valid_layouts(
    results: &[Row],
    reference: &Row,
    primary_tolerance: f64,
    secondary_tolerance: f64,
    min_distance: usize,
    max_valid: usize,
) -> (Vec<Row>, bool) {
    let rows = merge_results(results);
    let mut ref_row = rows
        .iter()
        .find(|r| r.layout == reference.layout)
        .cloned()
        .unwrap_or_else(|| {
            let mut r = reference.clone();
            r.final_score = r.secondary.unwrap_or(r.primary);
            r
        });
    ref_row.final_score = ref_row.secondary.unwrap_or(ref_row.primary);
    let mut rest: Vec<Row> = rows
        .iter()
        .filter(|r| r.layout != reference.layout)
        .cloned()
        .collect();
    rest.sort_by(|a, b| {
        a.primary
            .partial_cmp(&b.primary)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                a.secondary
                    .unwrap_or(f64::INFINITY)
                    .partial_cmp(&b.secondary.unwrap_or(f64::INFINITY))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });
    let mut selected: Vec<Row> = Vec::new();
    let mut truncated = false;
    let mut all = Vec::with_capacity(rest.len() + 1);
    all.push(ref_row.clone());
    all.extend(rest);
    for (idx, row) in all.into_iter().enumerate() {
        if idx != 0 && !comparable(&row, &ref_row, primary_tolerance, secondary_tolerance) {
            continue;
        }
        if selected
            .iter()
            .any(|kept| layout_distance(&row.layout, &kept.layout) < min_distance)
        {
            continue;
        }
        if max_valid > 0 && selected.len() >= max_valid {
            truncated = true;
            break;
        }
        selected.push(row);
    }
    (selected, truncated)
}

pub fn format_valid_layouts(rows: &[Row], status: &str, truncated: bool) -> Vec<String> {
    let mut lines = vec![format!("valid_layouts={} status={}", rows.len(), status)];
    for (i, row) in rows.iter().enumerate() {
        let sec_text = match row.secondary {
            Some(sec) => format!("{:.9}", sec),
            None => "nan".to_string(),
        };
        lines.push(format!(
            "  valid{}: primary={:.9} secondary={} {}",
            i + 1,
            row.primary,
            sec_text,
            row.layout
        ));
    }
    if truncated {
        lines.push("  additional comparable layouts omitted".to_string());
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(layout: &str) -> Row {
        Row::new("test", 1.0, None, layout)
    }

    #[test]
    fn letter_distance_ignores_punctuation() {
        let a = "abcdefghijklmnopqrstuvwxyz;,./";
        let b = "abcdefghijklmnopqrstuvwxyz,./;";
        assert_eq!(letter_distance(a, b), 0);
        assert_eq!(letter_distance(a, a), 0);
    }

    #[test]
    fn letter_distance_counts_displaced_letters() {
        let a = "abcdefghijklmnopqrstuvwxyz;,./";
        let b = "bacdefghijklmnopqrstuvwxyz;,./";
        assert_eq!(letter_distance(a, b), 2);
    }

    #[test]
    fn select_diverse_skips_near_duplicates() {
        let rows = vec![
            row("abcdefghijklmnopqrstuvwxyz;,./"),
            row("abcdefghijklmnopqrstuvwxyz,./;"),
            row("bacdefghijklmnopqrstuvwxyz;,./"),
            row("zyxwvutsrqponmlkjihgfedcba;,./"),
        ];
        let picked = select_diverse_layouts(&rows, 2, 4);
        assert_eq!(picked.len(), 2);
        assert_eq!(picked[0].layout, rows[0].layout);
        assert_eq!(picked[1].layout, rows[3].layout);
    }
}
