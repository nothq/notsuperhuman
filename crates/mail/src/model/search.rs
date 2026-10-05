use std::{error::Error, fmt, num::NonZeroU32};

use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

mod parser;
mod snippets;

pub use snippets::{MailSearchSnippet, MailSearchSnippetSegment, MailSearchSnippetText};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct MailSearchQuery {
    raw: String,
    expression: MailSearchExpression,
}

impl MailSearchQuery {
    pub fn as_str(&self) -> &str {
        self.raw.as_str()
    }

    pub fn expression(&self) -> &MailSearchExpression {
        &self.expression
    }

    pub(crate) fn contains_mailbox_predicate(&self) -> bool {
        expression_contains_mailbox_predicate(&self.expression)
    }

    pub(crate) fn inbox_scoped(inbox_mailbox_id: &str, split_query: &MailSearchQuery) -> Self {
        Self {
            raw: split_query.raw.clone(),
            expression: MailSearchExpression::All(vec![
                inbox_mailbox_expression(inbox_mailbox_id),
                split_query.expression.clone(),
            ]),
        }
    }

    pub(crate) fn inbox_other<'a>(
        inbox_mailbox_id: &str,
        enabled_queries: impl IntoIterator<Item = &'a MailSearchQuery>,
    ) -> Self {
        let enabled = enabled_queries
            .into_iter()
            .map(|query| query.expression.clone())
            .collect::<Vec<_>>();
        let mut expressions = vec![inbox_mailbox_expression(inbox_mailbox_id)];
        if !enabled.is_empty() {
            expressions.push(MailSearchExpression::Not(Box::new(
                MailSearchExpression::Any(enabled),
            )));
        }
        Self {
            raw: "Other".to_string(),
            expression: if expressions.len() == 1 {
                expressions
                    .pop()
                    .expect("Other always has an Inbox expression")
            } else {
                MailSearchExpression::All(expressions)
            },
        }
    }
}

fn inbox_mailbox_expression(mailbox_id: &str) -> MailSearchExpression {
    MailSearchExpression::Predicate(MailSearchPredicate::InMailbox(MailSearchMailboxSelector(
        mailbox_id.to_string(),
    )))
}

fn expression_contains_mailbox_predicate(expression: &MailSearchExpression) -> bool {
    match expression {
        MailSearchExpression::All(expressions) | MailSearchExpression::Any(expressions) => {
            expressions
                .iter()
                .any(expression_contains_mailbox_predicate)
        }
        MailSearchExpression::Not(expression) => expression_contains_mailbox_predicate(expression),
        MailSearchExpression::Predicate(MailSearchPredicate::InMailbox(_)) => true,
        MailSearchExpression::Predicate(_) => false,
    }
}

impl fmt::Display for MailSearchQuery {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl TryFrom<&str> for MailSearchQuery {
    type Error = MailSearchQueryError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl TryFrom<String> for MailSearchQuery {
    type Error = MailSearchQueryError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl Serialize for MailSearchQuery {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for MailSearchQuery {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(value).map_err(de::Error::custom)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum MailSearchExpression {
    All(Vec<Self>),
    Any(Vec<Self>),
    Not(Box<Self>),
    Predicate(MailSearchPredicate),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum MailSearchPredicate {
    Text(MailSearchText),
    From(MailSearchText),
    To(MailSearchText),
    Subject(MailSearchText),
    HasAttachment,
    InMailbox(MailSearchMailboxSelector),
    Unread,
    Starred,
    Shared,
    Before(MailSearchDate),
    After(MailSearchDate),
    OlderThan(MailSearchAge),
    NewerThan(MailSearchAge),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct MailSearchText {
    value: String,
    phrase: bool,
}

impl MailSearchText {
    pub fn value(&self) -> &str {
        self.value.as_str()
    }

    pub fn is_phrase(&self) -> bool {
        self.phrase
    }

    pub fn search_value(&self) -> String {
        if !self.phrase {
            return self.value.clone();
        }
        let escaped = self.value.replace('\\', "\\\\").replace('"', "\\\"");
        format!("\"{escaped}\"")
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct MailSearchMailboxSelector(String);

impl MailSearchMailboxSelector {
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MailSearchDate {
    year: i32,
    month: u8,
    day: u8,
}

impl MailSearchDate {
    pub fn year(self) -> i32 {
        self.year
    }

    pub fn month(self) -> u8 {
        self.month
    }

    pub fn day(self) -> u8 {
        self.day
    }

    pub fn iso_date(self) -> String {
        format!("{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MailSearchAge {
    amount: NonZeroU32,
    unit: MailSearchAgeUnit,
}

impl MailSearchAge {
    pub fn amount(self) -> NonZeroU32 {
        self.amount
    }

    pub fn unit(self) -> MailSearchAgeUnit {
        self.unit
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MailSearchAgeUnit {
    Days,
    Months,
    Years,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MailSearchQueryError {
    kind: MailSearchQueryErrorKind,
    byte_offset: usize,
}

impl MailSearchQueryError {
    fn new(kind: MailSearchQueryErrorKind, byte_offset: usize) -> Self {
        Self { kind, byte_offset }
    }

    pub fn kind(&self) -> &MailSearchQueryErrorKind {
        &self.kind
    }

    pub fn byte_offset(&self) -> usize {
        self.byte_offset
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MailSearchQueryErrorKind {
    Empty,
    UnterminatedQuote,
    MissingOperand,
    MissingValue { operator: String },
    InvalidDate { operator: String, value: String },
    InvalidRelativeAge { operator: String, value: String },
}

impl fmt::Display for MailSearchQueryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            MailSearchQueryErrorKind::Empty => {
                formatter.write_str("mail search query must not be empty")
            }
            MailSearchQueryErrorKind::UnterminatedQuote => write!(
                formatter,
                "mail search query has an unterminated quote at byte {}",
                self.byte_offset
            ),
            MailSearchQueryErrorKind::MissingOperand => write!(
                formatter,
                "mail search OR is missing an operand at byte {}",
                self.byte_offset
            ),
            MailSearchQueryErrorKind::MissingValue { operator } => write!(
                formatter,
                "mail search operator {operator}: is missing a value at byte {}",
                self.byte_offset
            ),
            MailSearchQueryErrorKind::InvalidDate { operator, value } => write!(
                formatter,
                "mail search operator {operator}: has invalid date {value:?} at byte {}; expected YYYY/MM/DD",
                self.byte_offset
            ),
            MailSearchQueryErrorKind::InvalidRelativeAge { operator, value } => write!(
                formatter,
                "mail search operator {operator}: has invalid age {value:?} at byte {}; expected a positive integer followed by d, m, or y",
                self.byte_offset
            ),
        }
    }
}

impl Error for MailSearchQueryError {}
