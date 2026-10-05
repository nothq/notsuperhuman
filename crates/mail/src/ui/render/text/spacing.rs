//! `letter-spacing`, emulated with Unicode spaces.

use crate::ui::render::{MailRenderOptions, MailTextFont};
use crate::ui::types::*;

/// `letter-spacing`, which GPUI cannot shape, emulated with hair spaces.
///
/// CSS adds the spacing after every character, the last one included. A hair
/// space is the narrowest space the font has, so the nearest whole number of
/// them after each character comes within half a hair space of the authored
/// tracking. Measuring the same string the renderer shapes keeps layout and
/// paint in agreement, whatever width the font gives the space.
pub(in crate::ui::render) fn spaced_text(
    text: &str,
    paragraph: &MailParagraph,
    run: &MailTextRun,
    font: &MailTextFont,
    options: &MailRenderOptions,
) -> String {
    let Some(spacing) = run.style.letter_spacing.or(paragraph.style.letter_spacing) else {
        return text.to_string();
    };
    if spacing <= 0.0 {
        return text.to_string();
    }
    // The tracking is built from the run's own font: a fixed string of
    // spaces — the widest that fits, then smaller ones for the remainder —
    // follows every character, and what is still owed is paid off with one
    // extra hair space now and then, so no gap differs from another by more
    // than a hair space while a line's total width comes out right. Paying
    // the whole tracking off with a running remainder gave some characters a
    // gap and others none, which read as different words.
    const SPACES: [char; 5] = ['\u{2004}', '\u{2005}', '\u{2006}', '\u{2009}', '\u{200A}'];
    let widths: Vec<(char, f32)> = SPACES
        .iter()
        .map(|space| (*space, (options.measure_text)(&space.to_string(), font)))
        .filter(|(_, advance)| *advance > 0.0)
        .collect();
    let Some((hair, hair_advance)) = widths.iter().copied().min_by(|a, b| a.1.total_cmp(&b.1))
    else {
        return text.to_string();
    };
    let (gap, gap_advance) = tracking_gap(spacing, &widths, hair_advance);
    if gap.is_empty() && spacing < hair_advance / 2.0 {
        return text.to_string();
    }
    let mut owed = 0.0f32;
    let mut spaced = String::with_capacity(text.len() * (gap.len() + 2));
    for character in text.chars() {
        spaced.push(character);
        if character == '\n' {
            owed = 0.0;
            continue;
        }
        spaced.push_str(&gap);
        owed += spacing - gap_advance;
        if owed >= hair_advance / 2.0 {
            spaced.push(hair);
            owed -= hair_advance;
        }
    }
    spaced
}

/// The spaces that follow every character: the widest that fit, then
/// narrower ones, until less than a hair space of the tracking is left.
fn tracking_gap(spacing: f32, widths: &[(char, f32)], hair_advance: f32) -> (String, f32) {
    let mut gap = String::new();
    let mut gap_advance = 0.0;
    while spacing - gap_advance >= hair_advance {
        let Some((space, advance)) = widths
            .iter()
            .copied()
            .find(|(_, advance)| *advance <= spacing - gap_advance)
        else {
            break;
        };
        gap.push(space);
        gap_advance += advance;
    }
    (gap, gap_advance)
}
