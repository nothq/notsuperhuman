//! Translates the app's search language into Gmail's, which it closely
//! mirrors: `from:`, `to:`, `subject:`, `has:attachment`, `is:unread`,
//! `before:`, `older_than:` and friends all exist on both sides.

use crate::model::{
    MailSearchAgeUnit, MailSearchExpression, MailSearchPredicate, MailSearchQuery, MailSearchText,
    Mailbox,
};

use super::convert::{ARCHIVE, DRAFT, INBOX, SENT, SNOOZED, SPAM, STARRED, TRASH};

/// The Gmail query for a search, or `None` when it can never match.
pub(crate) fn gmail_query(query: &MailSearchQuery, mailboxes: &[Mailbox]) -> Option<String> {
    expression_query(query.expression(), mailboxes)
}

/// The Gmail query that lists one folder.
pub(crate) fn mailbox_query(mailbox_id: &str, mailboxes: &[Mailbox]) -> String {
    match mailbox_id {
        INBOX => "in:inbox".to_string(),
        SENT => "in:sent".to_string(),
        DRAFT => "in:drafts".to_string(),
        SPAM => "in:spam".to_string(),
        TRASH => "in:trash".to_string(),
        STARRED => "is:starred".to_string(),
        "IMPORTANT" => "is:important".to_string(),
        ARCHIVE => "-in:inbox -in:drafts".to_string(),
        _ => mailboxes
            .iter()
            .find(|mailbox| mailbox.id == mailbox_id)
            .map(|mailbox| format!("label:{}", label_term(mailbox.name.as_str())))
            .unwrap_or_else(|| format!("label:{}", label_term(mailbox_id))),
    }
}

fn expression_query(expression: &MailSearchExpression, mailboxes: &[Mailbox]) -> Option<String> {
    match expression {
        MailSearchExpression::All(expressions) => {
            let terms = expressions
                .iter()
                .map(|expression| expression_query(expression, mailboxes))
                .collect::<Option<Vec<_>>>()?;
            Some(group(
                terms.into_iter().filter(|term| !term.is_empty()),
                " ",
            ))
        }
        MailSearchExpression::Any(expressions) => {
            let terms = expressions
                .iter()
                .filter_map(|expression| expression_query(expression, mailboxes))
                .collect::<Vec<_>>();
            if terms.is_empty() {
                return None;
            }
            if terms.iter().any(String::is_empty) {
                return Some(String::new());
            }
            Some(group(terms.into_iter(), " OR "))
        }
        MailSearchExpression::Not(expression) => match expression_query(expression, mailboxes) {
            None => Some(String::new()),
            Some(term) if term.is_empty() => None,
            Some(term) => Some(format!("-{}", parenthesize(term))),
        },
        MailSearchExpression::Predicate(predicate) => predicate_query(predicate, mailboxes),
    }
}

fn group(terms: impl Iterator<Item = String>, separator: &str) -> String {
    let terms = terms.collect::<Vec<_>>();
    match terms.len() {
        0 => String::new(),
        1 => terms.into_iter().next().unwrap_or_default(),
        _ => format!("({})", terms.join(separator)),
    }
}

fn parenthesize(term: String) -> String {
    if term.starts_with('(') || !term.contains(' ') {
        term
    } else {
        format!("({term})")
    }
}

fn predicate_query(predicate: &MailSearchPredicate, mailboxes: &[Mailbox]) -> Option<String> {
    let term = match predicate {
        MailSearchPredicate::Text(text) => text_term(text),
        MailSearchPredicate::From(text) => format!("from:{}", text_term(text)),
        MailSearchPredicate::To(text) => format!("to:{}", text_term(text)),
        MailSearchPredicate::Subject(text) => format!("subject:{}", text_term(text)),
        MailSearchPredicate::HasAttachment => "has:attachment".to_string(),
        MailSearchPredicate::InMailbox(selector) => {
            let selector = selector.as_str();
            let mailbox = mailboxes
                .iter()
                .find(|mailbox| mailbox.id == selector)
                .or_else(|| {
                    mailboxes.iter().find(|mailbox| {
                        mailbox
                            .role
                            .as_deref()
                            .is_some_and(|role| role.eq_ignore_ascii_case(selector))
                    })
                })
                .or_else(|| {
                    mailboxes
                        .iter()
                        .find(|mailbox| mailbox.name.eq_ignore_ascii_case(selector))
                });
            match mailbox {
                Some(mailbox) if mailbox.id == SNOOZED => return None,
                Some(mailbox) => mailbox_query(mailbox.id.as_str(), mailboxes),
                None => format!("label:{}", label_term(selector)),
            }
        }
        MailSearchPredicate::Unread => "is:unread".to_string(),
        MailSearchPredicate::Starred => "is:starred".to_string(),
        MailSearchPredicate::Shared => return None,
        MailSearchPredicate::Before(date) => format!(
            "before:{:04}/{:02}/{:02}",
            date.year(),
            date.month(),
            date.day()
        ),
        MailSearchPredicate::After(date) => format!(
            "after:{:04}/{:02}/{:02}",
            date.year(),
            date.month(),
            date.day()
        ),
        MailSearchPredicate::OlderThan(age) => {
            format!("older_than:{}{}", age.amount(), age_unit(age.unit()))
        }
        MailSearchPredicate::NewerThan(age) => {
            format!("newer_than:{}{}", age.amount(), age_unit(age.unit()))
        }
    };
    Some(term)
}

fn age_unit(unit: MailSearchAgeUnit) -> &'static str {
    match unit {
        MailSearchAgeUnit::Days => "d",
        MailSearchAgeUnit::Months => "m",
        MailSearchAgeUnit::Years => "y",
    }
}

fn text_term(text: &MailSearchText) -> String {
    let value = text.value();
    if text.is_phrase() || value.contains(char::is_whitespace) {
        format!("\"{}\"", value.replace('"', ""))
    } else {
        value.to_string()
    }
}

/// Gmail matches label names with spaces and slashes written as dashes.
fn label_term(name: &str) -> String {
    let term = name
        .trim()
        .to_lowercase()
        .chars()
        .map(|character| match character {
            ' ' | '/' | '&' | '(' | ')' | '"' => '-',
            other => other,
        })
        .collect::<String>();
    term
}

#[cfg(test)]
mod tests {
    use super::{gmail_query, mailbox_query};
    use crate::model::{MailSearchQuery, Mailbox};

    fn mailboxes() -> Vec<Mailbox> {
        vec![
            Mailbox {
                id: "INBOX".to_string(),
                name: "Inbox".to_string(),
                role: Some("inbox".to_string()),
                ..Mailbox::default()
            },
            Mailbox {
                id: "Label_7".to_string(),
                name: "Receipts/2026".to_string(),
                ..Mailbox::default()
            },
        ]
    }

    fn query(raw: &str) -> Option<String> {
        let query = MailSearchQuery::try_from(raw).expect("query");
        gmail_query(&query, &mailboxes())
    }

    #[test]
    fn operators_map_to_gmail() {
        assert_eq!(
            query("from:ada has:attachment is:unread").as_deref(),
            Some("(from:ada has:attachment is:unread)")
        );
        assert_eq!(
            query("subject:\"quarterly plan\"").as_deref(),
            Some("subject:\"quarterly plan\"")
        );
        assert_eq!(
            query("before:2026/01/02").as_deref(),
            Some("before:2026/01/02")
        );
    }

    #[test]
    fn mailboxes_resolve_by_role_and_name() {
        assert_eq!(query("in:inbox").as_deref(), Some("in:inbox"));
        assert_eq!(
            query("in:Receipts/2026").as_deref(),
            Some("label:receipts-2026")
        );
        assert_eq!(
            mailbox_query("ARCHIVE", &mailboxes()),
            "-in:inbox -in:drafts"
        );
    }

    #[test]
    fn alternatives_and_negations_group() {
        assert_eq!(
            query("from:ada OR from:bo").as_deref(),
            Some("(from:ada OR from:bo)")
        );
        assert_eq!(query("-from:ada").as_deref(), Some("-from:ada"));
    }
}
