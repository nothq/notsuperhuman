use std::{fmt, str::FromStr};

pub const MAIL_CONTACT_HISTORY_LIMIT: usize = 6;

const MAIL_CONTACT_ADDRESS_MAX_BYTES: usize = 254;
const MAIL_CONTACT_LOCAL_PART_MAX_BYTES: usize = 64;
const MAIL_CONTACT_DOMAIN_LABEL_MAX_BYTES: usize = 63;

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MailContactAddress(String);

impl MailContactAddress {
    pub fn parse(value: impl AsRef<str>) -> Result<Self, MailContactAddressError> {
        let value = value.as_ref().trim();
        validate_mail_contact_address(value)?;
        Ok(Self(value.to_ascii_lowercase()))
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Display for MailContactAddress {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for MailContactAddress {
    type Err = MailContactAddressError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
    }
}

impl TryFrom<String> for MailContactAddress {
    type Error = MailContactAddressError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl TryFrom<&str> for MailContactAddress {
    type Error = MailContactAddressError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MailContactAddressError {
    Empty,
    TooLong,
    InvalidSyntax,
}

impl fmt::Display for MailContactAddressError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Empty => "mail contact address must not be empty",
            Self::TooLong => "mail contact address is too long",
            Self::InvalidSyntax => "mail contact address has invalid syntax",
        })
    }
}

impl std::error::Error for MailContactAddressError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MailContactHistoryRequest {
    contact: MailContactAddress,
}

impl MailContactHistoryRequest {
    pub fn new(contact: MailContactAddress) -> Self {
        Self { contact }
    }

    pub fn contact(&self) -> &MailContactAddress {
        &self.contact
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MailContactHistoryPage {
    contact: MailContactAddress,
    entries: Vec<MailContactHistoryEntry>,
}

impl MailContactHistoryPage {
    pub fn new(
        contact: MailContactAddress,
        entries: Vec<MailContactHistoryEntry>,
    ) -> Result<Self, String> {
        if entries.len() > MAIL_CONTACT_HISTORY_LIMIT {
            return Err(format!(
                "mail contact history returned {} entries; limit is {MAIL_CONTACT_HISTORY_LIMIT}",
                entries.len()
            ));
        }
        Ok(Self { contact, entries })
    }

    pub fn contact(&self) -> &MailContactAddress {
        &self.contact
    }

    pub fn entries(&self) -> &[MailContactHistoryEntry] {
        &self.entries
    }

    pub fn into_entries(self) -> Vec<MailContactHistoryEntry> {
        self.entries
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MailContactHistoryEntry {
    pub thread_id: String,
    pub email_ids: Vec<String>,
    pub subject: String,
    pub preview: String,
    pub received_at: String,
}

fn validate_mail_contact_address(value: &str) -> Result<(), MailContactAddressError> {
    if value.is_empty() {
        return Err(MailContactAddressError::Empty);
    }
    if value.len() > MAIL_CONTACT_ADDRESS_MAX_BYTES {
        return Err(MailContactAddressError::TooLong);
    }
    if !value.is_ascii()
        || value
            .bytes()
            .any(|byte| byte.is_ascii_whitespace() || byte.is_ascii_control())
    {
        return Err(MailContactAddressError::InvalidSyntax);
    }
    let Some((local, domain)) = value.split_once('@') else {
        return Err(MailContactAddressError::InvalidSyntax);
    };
    if local.is_empty()
        || local.len() > MAIL_CONTACT_LOCAL_PART_MAX_BYTES
        || local.contains('@')
        || local.starts_with('.')
        || local.ends_with('.')
        || local.contains("..")
        || !local.bytes().all(valid_mail_contact_local_byte)
        || !valid_mail_contact_domain(domain)
    {
        return Err(MailContactAddressError::InvalidSyntax);
    }
    Ok(())
}

fn valid_mail_contact_local_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric()
        || matches!(
            byte,
            b'.' | b'!'
                | b'#'
                | b'$'
                | b'%'
                | b'&'
                | b'\''
                | b'*'
                | b'+'
                | b'-'
                | b'/'
                | b'='
                | b'?'
                | b'^'
                | b'_'
                | b'`'
                | b'{'
                | b'|'
                | b'}'
                | b'~'
        )
}

fn valid_mail_contact_domain(domain: &str) -> bool {
    !domain.is_empty()
        && !domain.starts_with('.')
        && !domain.ends_with('.')
        && domain.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= MAIL_CONTACT_DOMAIN_LABEL_MAX_BYTES
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        })
}
