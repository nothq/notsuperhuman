use std::ops::Range;

use crate::model::{MailSearchExpression, MailSearchPredicate, MailSearchQuery};

#[derive(Default)]
pub(super) struct MailSearchHighlightTerms {
    pub(super) text: Vec<String>,
    pub(super) from: Vec<String>,
    pub(super) to: Vec<String>,
    pub(super) subject: Vec<String>,
}

impl MailSearchHighlightTerms {
    pub(super) fn from_query(query: &MailSearchQuery) -> Self {
        let mut terms = Self::default();
        terms.collect(query.expression(), true);
        terms
    }

    fn collect(&mut self, expression: &MailSearchExpression, positive: bool) {
        match expression {
            MailSearchExpression::All(expressions) | MailSearchExpression::Any(expressions) => {
                for expression in expressions {
                    self.collect(expression, positive);
                }
            }
            MailSearchExpression::Not(expression) => self.collect(expression, !positive),
            MailSearchExpression::Predicate(predicate) if positive => match predicate {
                MailSearchPredicate::Text(text) => self.text.push(text.value().to_string()),
                MailSearchPredicate::From(text) => self.from.push(text.value().to_string()),
                MailSearchPredicate::To(text) => self.to.push(text.value().to_string()),
                MailSearchPredicate::Subject(text) => {
                    self.subject.push(text.value().to_string());
                }
                _ => {}
            },
            MailSearchExpression::Predicate(_) => {}
        }
    }
}

pub(super) fn mail_search_highlight_ranges(value: &str, terms: &[String]) -> Vec<Range<usize>> {
    let boundaries = value
        .char_indices()
        .map(|(byte_offset, _)| byte_offset)
        .chain(std::iter::once(value.len()))
        .collect::<Vec<_>>();
    let mut terms = terms
        .iter()
        .map(|term| term.trim())
        .filter(|term| !term.is_empty())
        .collect::<Vec<_>>();
    terms.sort_by_key(|term| std::cmp::Reverse(term.chars().count()));
    let mut ranges = Vec::new();
    for term in terms {
        append_mail_search_highlight_ranges(value, &boundaries, term, &mut ranges);
    }
    ranges.sort_by_key(|range| range.start);
    ranges
}

fn append_mail_search_highlight_ranges(
    value: &str,
    boundaries: &[usize],
    term: &str,
    ranges: &mut Vec<Range<usize>>,
) {
    let term_character_count = term.chars().count();
    if term_character_count == 0 || term_character_count >= boundaries.len() {
        return;
    }
    let lowercase_term = term.to_lowercase();
    for boundary_index in 0..boundaries.len() - term_character_count {
        let start = boundaries[boundary_index];
        let end = boundaries[boundary_index + term_character_count];
        if value[start..end].to_lowercase() != lowercase_term {
            continue;
        }
        if ranges
            .iter()
            .any(|range| start < range.end && range.start < end)
        {
            continue;
        }
        ranges.push(start..end);
    }
}
