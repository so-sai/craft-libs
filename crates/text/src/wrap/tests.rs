//! Architectural regression tests for Tier 5: Line Breaking (UAX #14) & Metric Hygiene.

use super::*;

/// Tests that empty string inputs return zero break opportunities without panicking.
#[test]
fn line_break_empty_string() {
    // -------------------------------------------------------------------------
    // WHY: Empty string is a baseline boundary case in all document layout engines.
    // DANGER: Calling next() on uninitialized cluster iterators can panic or return false offsets.
    // CONTRACT: Tier 5 - Invariant 3 (Zero-Allocation & Zero-Panic on empty input).
    // -------------------------------------------------------------------------
    let mut it = line_break_opportunities("");
    assert_eq!(it.next(), None);
}

/// Tests that standard ASCII words break exclusively at whitespace and mandatory line ends.
#[test]
fn line_break_ascii_words() {
    // -------------------------------------------------------------------------
    // WHY: Basic English / ASCII text must preserve standard UAX #14 word break opportunities.
    // DANGER: Over-filtering breaks could cause long ASCII lines to never wrap, causing text overflow.
    // CONTRACT: Tier 5 - General Line Breaking Conformance.
    // -------------------------------------------------------------------------
    let text = "hello world";
    let breaks: Vec<_> = line_break_opportunities(text).collect();
    // Offset 6 is after "hello ", offset 11 is end of text
    assert_eq!(breaks, vec![(6, BreakOpportunity::Allowed), (11, BreakOpportunity::Mandatory)]);
}

/// Tests that Vietnamese precomposed NFC text wraps cleanly at word boundaries.
#[test]
fn line_break_vietnamese_nfc() {
    // -------------------------------------------------------------------------
    // WHY: Standard precomposed Vietnamese words must break cleanly at spaces without breaking mid-word.
    // DANGER: Broken word offsets cause text layout engines to split syllables across line margins.
    // CONTRACT: Tier 5 - General Vietnamese Layout Invariant.
    // -------------------------------------------------------------------------
    let text = "Cộng hòa Xã hội";
    let breaks: Vec<_> = line_break_opportunities(text).collect();
    // Breaks occur at spaces and at the end of string
    assert_eq!(breaks.len(), 4);
    assert_eq!(breaks[0].1, BreakOpportunity::Allowed); // after "Cộng "
    assert_eq!(breaks[3].1, BreakOpportunity::Mandatory); // end of text
}

/// Tests that decomposed NFD Vietnamese strings strictly prohibit breaks inside combining marks.
#[test]
fn line_break_vietnamese_nfd_intra_cluster_invariant() {
    // -------------------------------------------------------------------------
    // WHY: Decomposed Vietnamese NFD stacks multiple combining marks:
    //      Base vowel + ccc 220 (dot below `\u{0323}`) + ccc 230 (circumflex `\u{0302}`).
    // DANGER: If a line break occurs inside "e\u{0323}\u{0302}", the diacritics wrap to the next line,
    //         leaving a naked 'e' at the end of the line and orphan diacritics at the start of the next.
    // CONTRACT: Tier 5 - Invariant 1 (Intra-Cluster Invariant).
    // -------------------------------------------------------------------------
    let nfd = "vie\u{0323}\u{0302}n khoa hoc";
    let breaks: Vec<_> = line_break_opportunities(nfd).collect();

    // Verify that NO break opportunity occurs strictly inside the NFD cluster [2..7]
    for (offset, _) in &breaks {
        assert!(*offset <= 2 || *offset >= 7, "Break opportunity at byte offset {} falls strictly inside NFD cluster [2..7]!", offset);
    }
}

/// Tests that Non-Breaking Spaces (U+00A0) strictly inhibit line breaks to keep phrases glued.
#[test]
fn line_break_nbsp_glue_semantics() {
    // -------------------------------------------------------------------------
    // WHY: Non-breaking space (NBSP `\u{00A0}`) is essential in Vietnamese administrative documents
    //      and technical blueprints to keep units and labels glued together (e.g. "100 kg", "Tầng 1").
    // DANGER: Breaking at NBSP separates numbers from their measurement units across lines.
    // CONTRACT: Tier 5 - Invariant 2 (Non-Breaking Space Adherence).
    // -------------------------------------------------------------------------
    let glued = "100\u{00A0}kg búa\u{00A0}tạ";
    let breaks: Vec<_> = line_break_opportunities(glued).collect();

    // Breaks must only occur at the standard ASCII space and at the end of text; NEVER at NBSP
    for (offset, opp) in &breaks {
        if *opp == BreakOpportunity::Allowed {
            let ch = glued.get(*offset..).and_then(|s| s.chars().next());
            assert_ne!(ch, Some('\u{00A0}'), "Allowed break must not precede NBSP at offset {}", offset);
        }
    }
    // "100\u{00A0}kg " has an allowed break after the regular space
    assert!(breaks.iter().any(|(offset, opp)| *opp == BreakOpportunity::Allowed && glued[..*offset].ends_with(' ')));
}

/// Tests that Narrow Non-Breaking Spaces (U+202F) strictly inhibit line breaks.
#[test]
fn line_break_narrow_nbsp() {
    // -------------------------------------------------------------------------
    // WHY: Narrow NBSP (`\u{202F}`) is standard in typographic layout for thousands separators and thin units.
    // DANGER: Breaking across narrow NBSP breaks numbers and formulas.
    // CONTRACT: Tier 5 - Invariant 2 (NBSP Glue Semantics for Narrow Spaces).
    // -------------------------------------------------------------------------
    let text = "10\u{202F}000 VND";
    let breaks: Vec<_> = line_break_opportunities(text).collect();

    // Must not break at the narrow NBSP between "10" and "000"
    for (offset, _) in &breaks {
        assert_ne!(*offset, 2, "Break must not occur at Narrow NBSP offset 2!");
    }
}

/// Tests that Emoji ZWJ sequences and modifiers never break across line margins.
#[test]
fn line_break_emoji_zwj_and_modifiers() {
    // -------------------------------------------------------------------------
    // WHY: Emoji sequences (such as family ZWJ `👨\u{200D}👩\u{200D}👧\u{200D}👦` or skin-tone modifiers `👍🏽`) form
    //      indivisible visual clusters.
    // DANGER: Line breaks inside an emoji sequence render broken surrogate icons or naked gender signs.
    // CONTRACT: Tier 5 - Invariant 1 (Intra-Cluster Invariant for Extended Scripts).
    // -------------------------------------------------------------------------
    let family = "👨\u{200D}👩\u{200D}👧\u{200D}👦";
    let text = format!("icon: {family} text");
    let breaks: Vec<_> = line_break_opportunities(&text).collect();

    // Family emoji occupies bytes [6..31]. Ensure zero breaks within (6..31)
    for (offset, _) in &breaks {
        assert!(*offset <= 6 || *offset >= 31, "Break at offset {} broke an emoji ZWJ cluster!", offset);
    }
}

/// Metamorphic sweep verifying that all break offsets strictly align with valid grapheme cluster boundaries.
#[test]
fn line_break_metamorphic_grapheme_boundary_sweep() {
    // -------------------------------------------------------------------------
    // WHY: Metamorphic testing across heterogeneous multilingual corpora verifies that every
    //      single emitted break opportunity is a verified cluster boundary.
    // DANGER: Off-by-one errors in line break iterators cause panic slicing in downstream text layout.
    // CONTRACT: Tier 5 - Invariant 1 & Invariant 3 (Mathematical Cluster Alignment).
    // -------------------------------------------------------------------------
    let samples = [
        "Cộng hòa Xã hội Chủ nghĩa Việt Nam",
        "vie\u{0323}\u{0302}n nghie\u{0302}n c\u{031B}\u{0301}u",
        "100\u{00A0}kg\t200\u{202F}m\n300\u{00A0}N",
        "👨\u{200D}👩\u{200D}👧\u{200D}👦 và 🇻🇳 cờ đỏ sao vàng",
        "Line 1\r\nLine 2\r\nLine 3\n",
    ];

    for &sample in &samples {
        // Collect all valid cluster boundary offsets using UAX #29
        let mut valid_boundaries = std::collections::BTreeSet::new();
        valid_boundaries.insert(0);
        let mut curr = 0;
        for cluster in unicode_segmentation::UnicodeSegmentation::graphemes(sample, true) {
            curr += cluster.len();
            valid_boundaries.insert(curr);
        }

        // Assert every break opportunity lands precisely on a valid cluster boundary
        for (offset, _) in line_break_opportunities(sample) {
            assert!(valid_boundaries.contains(&offset), "Break opportunity offset {} in {:?} does not land on a grapheme boundary!", offset, sample);
        }
    }
}
