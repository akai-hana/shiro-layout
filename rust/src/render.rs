//! Render a 30-key layout as staggered ASCII rows.

pub const ROW_LENGTH: usize = 10;
pub const LAYOUT_LENGTH: usize = 30;

pub fn render(layout: &str, fingers: Option<&[char]>) -> Result<String, String> {
    let chars: Vec<char> = layout.chars().collect();
    if chars.len() != LAYOUT_LENGTH {
        return Err(format!(
            "layout must be {} characters, got {}",
            LAYOUT_LENGTH,
            chars.len()
        ));
    }
    let mut rows: Vec<Vec<String>> = chars
        .chunks(ROW_LENGTH)
        .map(|row| row.iter().map(|c| c.to_string()).collect())
        .collect();
    if let Some(fingers) = fingers {
        if fingers.len() != LAYOUT_LENGTH {
            return Err(format!(
                "fingers must be {} entries, got {}",
                LAYOUT_LENGTH,
                fingers.len()
            ));
        }
        for (r, row) in rows.iter_mut().enumerate() {
            for (c, cell) in row.iter_mut().enumerate() {
                cell.push(fingers[r * ROW_LENGTH + c]);
            }
        }
    }
    Ok(rows
        .iter()
        .enumerate()
        .map(|(i, row)| {
            format!(
                "{}{}  {}",
                " ".repeat(i),
                row[..5].join(" "),
                row[5..].join(" ")
            )
        })
        .collect::<Vec<_>>()
        .join("\n"))
}
