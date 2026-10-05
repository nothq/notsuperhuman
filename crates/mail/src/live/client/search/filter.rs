use crate::model::{
    MailSearchAge, MailSearchAgeUnit, MailSearchDate, MailSearchExpression, MailSearchPredicate,
    MailSearchText,
};
use serde_json::{json, Value};
use time::{format_description::well_known::Rfc3339, Date, Duration, Month, OffsetDateTime};

use crate::live::types::JmapMailbox;

pub(super) fn expression_uses_mailbox(expression: &MailSearchExpression) -> bool {
    match expression {
        MailSearchExpression::All(expressions) | MailSearchExpression::Any(expressions) => {
            expressions.iter().any(expression_uses_mailbox)
        }
        MailSearchExpression::Not(expression) => expression_uses_mailbox(expression),
        MailSearchExpression::Predicate(MailSearchPredicate::InMailbox(_)) => true,
        MailSearchExpression::Predicate(_) => false,
    }
}

pub(super) fn constant_search_truth(expression: &MailSearchExpression) -> Option<bool> {
    match expression {
        MailSearchExpression::All(expressions) => {
            let mut all_constant = true;
            for expression in expressions {
                match constant_search_truth(expression) {
                    Some(false) => return Some(false),
                    Some(true) => {}
                    None => all_constant = false,
                }
            }
            all_constant.then_some(true)
        }
        MailSearchExpression::Any(expressions) => {
            let mut all_constant = true;
            for expression in expressions {
                match constant_search_truth(expression) {
                    Some(true) => return Some(true),
                    Some(false) => {}
                    None => all_constant = false,
                }
            }
            all_constant.then_some(false)
        }
        MailSearchExpression::Not(expression) => {
            constant_search_truth(expression).map(|value| !value)
        }
        MailSearchExpression::Predicate(MailSearchPredicate::Shared) => Some(false),
        MailSearchExpression::Predicate(_) => None,
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(super) enum ResolvedSearchFilter {
    Always,
    Never,
    Filter(Value),
}

impl ResolvedSearchFilter {
    fn all(filters: impl IntoIterator<Item = Self>) -> Self {
        let mut conditions = Vec::new();
        for filter in filters {
            match filter {
                Self::Always => {}
                Self::Never => return Self::Never,
                Self::Filter(condition) => conditions.push(condition),
            }
        }
        match conditions.len() {
            0 => Self::Always,
            1 => Self::Filter(conditions.pop().expect("single JMAP search condition")),
            _ => Self::Filter(json!({ "operator": "AND", "conditions": conditions })),
        }
    }

    fn any(filters: impl IntoIterator<Item = Self>) -> Self {
        let mut conditions = Vec::new();
        for filter in filters {
            match filter {
                Self::Always => return Self::Always,
                Self::Never => {}
                Self::Filter(condition) => conditions.push(condition),
            }
        }
        match conditions.len() {
            0 => Self::Never,
            1 => Self::Filter(conditions.pop().expect("single JMAP search condition")),
            _ => Self::Filter(json!({ "operator": "OR", "conditions": conditions })),
        }
    }

    fn not(self) -> Self {
        match self {
            Self::Always => Self::Never,
            Self::Never => Self::Always,
            Self::Filter(condition) => {
                Self::Filter(json!({ "operator": "NOT", "conditions": [condition] }))
            }
        }
    }

    pub(super) fn into_jmap_filter(self) -> Value {
        match self {
            Self::Always => json!({}),
            Self::Filter(filter) => filter,
            Self::Never => unreachable!("locally empty searches do not reach JMAP"),
        }
    }
}

pub(super) fn resolve_search_filter(
    expression: &MailSearchExpression,
    mailboxes: &[JmapMailbox],
    now: OffsetDateTime,
) -> Result<ResolvedSearchFilter, String> {
    match expression {
        MailSearchExpression::All(expressions) => {
            let mut filters = Vec::new();
            for expression in expressions {
                let filter = resolve_search_filter(expression, mailboxes, now)?;
                if filter == ResolvedSearchFilter::Never {
                    return Ok(ResolvedSearchFilter::Never);
                }
                filters.push(filter);
            }
            Ok(ResolvedSearchFilter::all(filters))
        }
        MailSearchExpression::Any(expressions) => {
            let mut filters = Vec::new();
            for expression in expressions {
                let filter = resolve_search_filter(expression, mailboxes, now)?;
                if filter == ResolvedSearchFilter::Always {
                    return Ok(ResolvedSearchFilter::Always);
                }
                filters.push(filter);
            }
            Ok(ResolvedSearchFilter::any(filters))
        }
        MailSearchExpression::Not(expression) => {
            resolve_search_filter(expression, mailboxes, now).map(ResolvedSearchFilter::not)
        }
        MailSearchExpression::Predicate(predicate) => {
            resolve_search_predicate(predicate, mailboxes, now)
        }
    }
}

fn resolve_search_predicate(
    predicate: &MailSearchPredicate,
    mailboxes: &[JmapMailbox],
    now: OffsetDateTime,
) -> Result<ResolvedSearchFilter, String> {
    let filter = match predicate {
        MailSearchPredicate::Text(text) => text_filter("text", text),
        MailSearchPredicate::From(text) => text_filter("from", text),
        MailSearchPredicate::To(text) => text_filter("to", text),
        MailSearchPredicate::Subject(text) => text_filter("subject", text),
        MailSearchPredicate::HasAttachment => json!({ "hasAttachment": true }),
        MailSearchPredicate::InMailbox(selector) => {
            let mailbox_id = mailboxes
                .iter()
                .find(|mailbox| mailbox.id == selector.as_str())
                .or_else(|| {
                    mailboxes.iter().find(|mailbox| {
                        mailbox
                            .role
                            .as_deref()
                            .is_some_and(|role| role.eq_ignore_ascii_case(selector.as_str()))
                    })
                })
                .or_else(|| {
                    mailboxes
                        .iter()
                        .find(|mailbox| mailbox.name.eq_ignore_ascii_case(selector.as_str()))
                })
                .map(|mailbox| mailbox.id.as_str())
                .ok_or_else(|| format!("missing JMAP mailbox {}", selector.as_str()))?;
            json!({ "inMailbox": mailbox_id })
        }
        MailSearchPredicate::Unread => json!({ "notKeyword": "$seen" }),
        MailSearchPredicate::Starred => json!({ "hasKeyword": "$flagged" }),
        MailSearchPredicate::Shared => return Ok(ResolvedSearchFilter::Never),
        MailSearchPredicate::Before(date) => {
            json!({ "before": date_boundary(*date)? })
        }
        MailSearchPredicate::After(date) => {
            json!({ "after": date_boundary(*date)? })
        }
        MailSearchPredicate::OlderThan(age) => {
            json!({ "before": relative_boundary(now, *age)? })
        }
        MailSearchPredicate::NewerThan(age) => {
            json!({ "after": relative_boundary(now, *age)? })
        }
    };
    Ok(ResolvedSearchFilter::Filter(filter))
}

fn text_filter(property: &str, text: &MailSearchText) -> Value {
    let mut filter = serde_json::Map::new();
    filter.insert(property.to_string(), Value::String(text.search_value()));
    Value::Object(filter)
}

fn date_boundary(date: MailSearchDate) -> Result<String, String> {
    let month = Month::try_from(date.month())
        .map_err(|error| format!("invalid parsed mail search month: {error}"))?;
    Date::from_calendar_date(date.year(), month, date.day())
        .map_err(|error| format!("invalid parsed mail search date: {error}"))?
        .midnight()
        .assume_utc()
        .format(&Rfc3339)
        .map_err(|error| format!("failed to format mail search date: {error}"))
}

fn relative_boundary(now: OffsetDateTime, age: MailSearchAge) -> Result<String, String> {
    let amount = i64::from(age.amount().get());
    let boundary = match age.unit() {
        MailSearchAgeUnit::Days => now.checked_sub(Duration::days(amount)),
        MailSearchAgeUnit::Months => subtract_calendar_months(now, amount),
        MailSearchAgeUnit::Years => amount
            .checked_mul(12)
            .and_then(|months| subtract_calendar_months(now, months)),
    };
    boundary
        .ok_or_else(|| "mail search relative date is outside the supported range".to_string())?
        .format(&Rfc3339)
        .map_err(|error| format!("failed to format relative mail search date: {error}"))
}

fn subtract_calendar_months(now: OffsetDateTime, months: i64) -> Option<OffsetDateTime> {
    let current_month = i64::from(u8::from(now.month())) - 1;
    let target_month_index = i64::from(now.year())
        .checked_mul(12)?
        .checked_add(current_month)?
        .checked_sub(months)?;
    let target_year = i32::try_from(target_month_index.div_euclid(12)).ok()?;
    let target_month = u8::try_from(target_month_index.rem_euclid(12) + 1)
        .ok()
        .and_then(|month| Month::try_from(month).ok())?;
    let target_day = now.day().min(target_month.length(target_year));
    let target_date = Date::from_calendar_date(target_year, target_month, target_day).ok()?;
    Some(now.replace_date(target_date))
}
