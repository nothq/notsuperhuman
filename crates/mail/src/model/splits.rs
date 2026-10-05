use std::{error::Error, fmt};

use super::MailSearchQuery;

pub const MAIL_SPLIT_MAX_COUNT: usize = 16;
pub const MAIL_SPLIT_NAME_MAX_CHARS: usize = 40;
pub const MAIL_SPLIT_NAME_MAX_BYTES: usize = 160;
pub const MAIL_SPLIT_QUERY_MAX_BYTES: usize = 2 * 1024;

const MAIL_SPLIT_ID_HEX_BYTES: usize = 64;

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MailSplitId(String);

impl MailSplitId {
    pub fn parse(value: impl Into<String>) -> Result<Self, MailSplitValidationError> {
        let value = value.into();
        if value.len() != MAIL_SPLIT_ID_HEX_BYTES
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(MailSplitValidationError::InvalidId);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Display for MailSplitId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MailSplitName(String);

impl MailSplitName {
    pub fn parse(value: impl AsRef<str>) -> Result<Self, MailSplitValidationError> {
        let value = value.as_ref().trim();
        if value.is_empty() {
            return Err(MailSplitValidationError::EmptyName);
        }
        if value.len() > MAIL_SPLIT_NAME_MAX_BYTES
            || value.chars().count() > MAIL_SPLIT_NAME_MAX_CHARS
        {
            return Err(MailSplitValidationError::NameTooLong);
        }
        Ok(Self(value.to_string()))
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MailSplitEnabled {
    Disabled,
    Enabled,
}

impl MailSplitEnabled {
    pub const fn is_enabled(self) -> bool {
        matches!(self, Self::Enabled)
    }
}

impl From<bool> for MailSplitEnabled {
    fn from(value: bool) -> Self {
        if value {
            Self::Enabled
        } else {
            Self::Disabled
        }
    }
}

impl From<MailSplitEnabled> for bool {
    fn from(value: MailSplitEnabled) -> Self {
        value.is_enabled()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct MailSplitOrder(u32);

impl MailSplitOrder {
    pub(crate) const fn new(value: u32) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u32 {
        self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MailSplitDraft {
    name: MailSplitName,
    query: MailSearchQuery,
}

impl MailSplitDraft {
    pub fn parse(
        name: impl AsRef<str>,
        query: impl AsRef<str>,
    ) -> Result<Self, MailSplitValidationError> {
        let name = MailSplitName::parse(name)?;
        let raw_query = query.as_ref();
        if raw_query.len() > MAIL_SPLIT_QUERY_MAX_BYTES {
            return Err(MailSplitValidationError::QueryTooLong);
        }
        let query = MailSearchQuery::parse(raw_query)
            .map_err(|error| MailSplitValidationError::InvalidQuery(error.to_string()))?;
        if query.contains_mailbox_predicate() {
            return Err(MailSplitValidationError::MailboxPredicate);
        }
        Ok(Self { name, query })
    }

    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn query(&self) -> &MailSearchQuery {
        &self.query
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MailSplitDefinition {
    id: MailSplitId,
    name: MailSplitName,
    query: MailSearchQuery,
    order: MailSplitOrder,
    enabled: MailSplitEnabled,
}

impl MailSplitDefinition {
    pub(crate) fn from_parts(
        id: MailSplitId,
        draft: MailSplitDraft,
        order: MailSplitOrder,
        enabled: MailSplitEnabled,
    ) -> Self {
        Self {
            id,
            name: draft.name,
            query: draft.query,
            order,
            enabled,
        }
    }

    pub fn id(&self) -> &MailSplitId {
        &self.id
    }

    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn query(&self) -> &MailSearchQuery {
        &self.query
    }

    pub const fn order(&self) -> MailSplitOrder {
        self.order
    }

    pub const fn enabled(&self) -> MailSplitEnabled {
        self.enabled
    }

    pub const fn is_enabled(&self) -> bool {
        self.enabled.is_enabled()
    }

    pub(crate) fn replace_draft(&mut self, draft: MailSplitDraft) {
        self.name = draft.name;
        self.query = draft.query;
    }

    pub(crate) fn set_enabled(&mut self, enabled: MailSplitEnabled) {
        self.enabled = enabled;
    }

    pub(crate) fn set_order(&mut self, order: MailSplitOrder) {
        self.order = order;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MailSplitMove {
    Earlier,
    Later,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MailSplitMutation {
    Create(MailSplitDraft),
    Edit {
        id: MailSplitId,
        draft: MailSplitDraft,
    },
    SetEnabled {
        id: MailSplitId,
        enabled: MailSplitEnabled,
    },
    Move {
        id: MailSplitId,
        direction: MailSplitMove,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MailSplitValidationError {
    InvalidId,
    EmptyName,
    NameTooLong,
    QueryTooLong,
    InvalidQuery(String),
    MailboxPredicate,
}

impl fmt::Display for MailSplitValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidId => formatter.write_str("Mail split ID is invalid"),
            Self::EmptyName => formatter.write_str("Split name is required"),
            Self::NameTooLong => write!(
                formatter,
                "Split name must be at most {MAIL_SPLIT_NAME_MAX_CHARS} characters"
            ),
            Self::QueryTooLong => write!(
                formatter,
                "Split query must be at most {MAIL_SPLIT_QUERY_MAX_BYTES} bytes"
            ),
            Self::InvalidQuery(error) => write!(formatter, "Invalid split query: {error}"),
            Self::MailboxPredicate => formatter.write_str(
                "Split queries cannot contain in: mailbox filters; splits are always Inbox-scoped",
            ),
        }
    }
}

impl Error for MailSplitValidationError {}
