//! Line breaking primitives compliant with Unicode Standard Annex #14 (UAX #14)
//! and hardened with Extended Grapheme Cluster invariants (UAX #29).
//!
//! # Architecture Context (Tier 5: Layout & Metric Hygiene)
//! - **WHY:** Standard line breakers (UAX #14) can emit break opportunities inside multi-codepoint
//!   grapheme clusters (such as decomposed Vietnamese NFD vowels with stacked accents or emoji ZWJ).
//!   This module guarantees that break opportunities strictly align with UAX #29 cluster boundaries.
//! - **DANGER:** Breaking inside a cluster isolates combining diacritics onto a new line as orphan
//!   marks, or breaks glued measurements (`100 kg`, `Tầng 1`) connected by non-breaking spaces.
//! - **CONTRACT:**
//!   1. Intra-Cluster Invariant: Zero break opportunities strictly inside an Extended Grapheme Cluster.
//!   2. NBSP Glue Semantics: `U+00A0` and `U+202F` strictly inhibit break opportunities before and after.
//!   3. Zero-Allocation: Operates directly over `&str` with $O(1)$ stack space.

pub use unicode_linebreak::BreakOpportunity;
use unicode_segmentation::{GraphemeIndices, UnicodeSegmentation};

#[cfg(test)]
mod tests;

/// A zero-allocation iterator over line break opportunities compliant with UAX #14
/// and strictly aligned to Extended Grapheme Cluster boundaries (UAX #29).
pub struct LineBreakIterator<'a, I> {
    /// Full borrowed text slice being wrapped.
    text: &'a str,
    /// Underlying UAX #14 line breaking iterator.
    linebreaks: I,
    /// Underlying UAX #29 extended grapheme cluster iterator.
    graphemes: GraphemeIndices<'a>,
    /// Cached enclosing grapheme cluster `(start, end)` byte offsets.
    current_cluster: Option<(usize, usize)>,
}

impl<'a, I> LineBreakIterator<'a, I>
where
    I: Iterator<Item = (usize, BreakOpportunity)>,
{
    /// Creates a new `LineBreakIterator` wrapping the given line breaks iterator.
    ///
    /// # Safety & Invariants
    /// - Zero heap allocations.
    /// - Zero panics.
    pub fn new(text: &'a str, linebreaks: I) -> Self {
        let mut graphemes = text.grapheme_indices(true);
        let current_cluster = graphemes.next().map(|(start, cluster)| (start, start + cluster.len()));
        Self { text, linebreaks, graphemes, current_cluster }
    }
}

impl<'a, I> Iterator for LineBreakIterator<'a, I>
where
    I: Iterator<Item = (usize, BreakOpportunity)>,
{
    type Item = (usize, BreakOpportunity);

    fn next(&mut self) -> Option<Self::Item> {
        if self.text.is_empty() {
            return None;
        }

        for (offset, opp) in self.linebreaks.by_ref() {
            // NBSP Glue Semantics: soft break opportunities adjacent to NBSP/NNBSP are inhibited.
            if opp == BreakOpportunity::Allowed {
                // Inhibit break immediately preceding U+00A0 (NBSP) or U+202F (NNBSP)
                if let Some(ch) = self.text.get(offset..).and_then(|s| s.chars().next())
                    && (ch == '\u{00A0}' || ch == '\u{202F}')
                {
                    continue;
                }
                // Inhibit break immediately following U+00A0 (NBSP) or U+202F (NNBSP)
                if let Some(ch) = self.text.get(..offset).and_then(|s| s.chars().next_back())
                    && (ch == '\u{00A0}' || ch == '\u{202F}')
                {
                    continue;
                }
            }

            // Advance grapheme clusters until current_cluster ends at or after the break offset
            while let Some((_, end)) = self.current_cluster {
                if end < offset {
                    self.current_cluster = self.graphemes.next().map(|(start, cluster)| (start, start + cluster.len()));
                } else {
                    break;
                }
            }

            // Check cluster boundary alignment
            if let Some((start, end)) = self.current_cluster {
                // Allowed break point: must be at a grapheme boundary (either cluster start or cluster end)
                if offset == start || offset == end {
                    return Some((offset, opp));
                }
                // Intra-Cluster Invariant: start < offset < end lies strictly inside a grapheme cluster.
                // Suppress this break opportunity to prevent diacritic peeling across lines.
                continue;
            } else if offset == self.text.len() {
                // End-of-string boundary
                return Some((offset, opp));
            }
        }
        None
    }
}

/// Returns a zero-allocation iterator yielding `(byte_offset, BreakOpportunity)` for `text`.
///
/// Complies with UAX #14 while strictly enforcing UAX #29 cluster boundaries and NBSP glue semantics.
pub fn line_break_opportunities(text: &str) -> LineBreakIterator<'_, impl Iterator<Item = (usize, BreakOpportunity)> + Clone + '_> {
    LineBreakIterator::new(text, unicode_linebreak::linebreaks(text))
}
