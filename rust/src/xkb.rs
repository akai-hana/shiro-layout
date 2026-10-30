//! Generate XKB symbols files from a 30-character layout string.
//!
//! Port of `shirolyzer/utils/xkb.py`.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};

pub const ROWS: [&str; 3] = ["QWERTYUIOP", "ASDFGHJKL;", "ZXCVBNM,./"];
pub const LAYOUT_LENGTH: usize = 30;
pub const LETTERS: &str = "abcdefghijklmnopqrstuvwxyz";

const SYMBOLS: [(&str, (&str, &str)); 12] = [
    (";", ("semicolon", "colon")),
    (",", ("comma", "less")),
    (".", ("period", "greater")),
    ("/", ("slash", "question")),
    ("'", ("apostrophe", "quotedbl")),
    ("-", ("minus", "underscore")),
    ("=", ("equal", "plus")),
    ("[", ("bracketleft", "braceleft")),
    ("]", ("bracketright", "braceright")),
    ("\\", ("backslash", "bar")),
    ("`", ("grave", "asciitilde")),
    ("", ("", "")),
];

const EXTRA_KEYS: [(&str, (&str, &str)); 2] = [
    ("AC11", ("apostrophe", "quotedbl")),
    ("LSGT", ("apostrophe", "quotedbl")),
];

fn keycode(key: char) -> Option<String> {
    for (row, prefix) in ROWS.iter().zip(["AD", "AC", "AB"]) {
        if let Some(column) = row.chars().position(|c| c == key) {
            return Some(format!("{}{:02}", prefix, column + 1));
        }
    }
    None
}

fn symbol_for(ch: char) -> (String, String) {
    for (key, pair) in SYMBOLS.iter() {
        if key.len() == 1 && key.starts_with(ch) {
            return (pair.0.to_string(), pair.1.to_string());
        }
    }
    let lower = ch.to_ascii_lowercase();
    let upper = lower.to_uppercase().next().unwrap_or(lower);
    (lower.to_string(), upper.to_string())
}

fn normalise(layout: &str) -> Result<String> {
    let chars: Vec<char> = layout.chars().collect();
    if chars.len() != LAYOUT_LENGTH {
        return Err(anyhow!(
            "layout must be {} characters, got {}",
            LAYOUT_LENGTH,
            chars.len()
        ));
    }
    let layout = layout.to_lowercase();
    let valid: HashSet<char> = SYMBOLS
        .iter()
        .filter(|(k, _)| k.len() == 1)
        .map(|(k, _)| k.chars().next().unwrap())
        .chain(LETTERS.chars())
        .collect();
    for ch in layout.chars() {
        if !valid.contains(&ch) {
            return Err(anyhow!("unsupported layout character: {:?}", ch));
        }
    }
    if layout.chars().collect::<HashSet<_>>().len() != LAYOUT_LENGTH {
        return Err(anyhow!("layout contains duplicate characters"));
    }
    let present: HashSet<char> = layout.chars().collect();
    let mut missing: Vec<char> = LETTERS.chars().filter(|c| !present.contains(c)).collect();
    missing.sort_unstable();
    if !missing.is_empty() {
        return Err(anyhow!(
            "layout is missing letters: {}",
            missing.into_iter().collect::<String>()
        ));
    }
    Ok(layout)
}

pub fn safe_name(layout: &str) -> Result<String> {
    let layout = normalise(layout)?;
    let digest = sha1_hex(layout.as_bytes());
    Ok(format!("shiro_{}", &digest[..8]))
}

fn sha1_hex(data: &[u8]) -> String {
    let mut h: [u32; 5] = [
        0x67452301,
        0xEFCDAB89,
        0x98BADCFE,
        0x10325476,
        0xC3D2E1F0,
    ];
    let bit_len = (data.len() as u64) * 8;
    let mut msg = data.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());
    let mut w = [0u32; 80];
    for chunk in msg.chunks(64) {
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                chunk[i * 4],
                chunk[i * 4 + 1],
                chunk[i * 4 + 2],
                chunk[i * 4 + 3],
            ]);
        }
        for i in 16..80 {
            w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
        }
        let (mut a, mut b, mut c, mut d, mut e) = (h[0], h[1], h[2], h[3], h[4]);
        for i in 0..80 {
            let (f, k) = match i {
                0..=19 => ((b & c) | ((!b) & d), 0x5A827999u32),
                20..=39 => (b ^ c ^ d, 0x6ED9EBA1u32),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1BBCDCu32),
                _ => (b ^ c ^ d, 0xCA62C1D6u32),
            };
            let temp = a
                .rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k)
                .wrapping_add(w[i]);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = temp;
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
    }
    h.iter().map(|v| format!("{:08x}", v)).collect()
}

pub fn symbols(layout: &str, name: &str) -> Result<String> {
    let layout = normalise(layout)?;
    let name = name.trim();
    if name.is_empty() {
        return Err(anyhow!("layout name must not be empty"));
    }
    let chars: Vec<char> = layout.chars().collect();
    let rows = [&chars[0..10], &chars[10..20], &chars[20..30]];
    let join = |row: &[char]| row.iter().map(|c| c.to_string()).collect::<Vec<_>>().join(" ");
    let mut lines = vec![
        format!("// {} layout — generated by shirolyzer", name),
        format!("// top:  {}  {}", join(&rows[0][..5]), join(&rows[0][5..])),
        format!("// home: {}  {}", join(&rows[1][..5]), join(&rows[1][5..])),
        format!("// bot:  {}  {}", join(&rows[2][..5]), join(&rows[2][5..])),
        "partial alphanumeric_keys".to_string(),
        "xkb_symbols \"basic\" {".to_string(),
        format!("    name[Group1] = \"{}\";", name),
        String::new(),
        "    include \"us(basic)\"".to_string(),
        String::new(),
    ];
    for (row_index, row) in rows.iter().enumerate() {
        for (column, &ch) in row.iter().enumerate() {
            let key = ROWS[row_index].chars().nth(column).unwrap();
            let code = keycode(key).unwrap();
            let (base, shift) = symbol_for(ch);
            let base_field = format!("{},", base);
            lines.push(format!(
                "    key <{}> {{ [ {:<13}{:<9} ] }};",
                code, base_field, shift
            ));
        }
        lines.push(String::new());
    }
    for (code, (base, shift)) in EXTRA_KEYS.iter() {
        let base_field = format!("{},", base);
        lines.push(format!(
            "    key <{}> {{ [ {:<13}{:<9} ] }};",
            code, base_field, shift
        ));
    }
    lines.push("};".to_string());
    lines.push(String::new());
    Ok(lines.join("\n"))
}

pub fn write(path: &Path, layout: &str, name: &str) -> Result<PathBuf> {
    let path = path.to_path_buf();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, symbols(layout, name)?)?;
    Ok(path)
}
