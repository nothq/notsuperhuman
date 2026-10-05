use std::num::NonZeroU32;

use super::{
    MailSearchAge, MailSearchAgeUnit, MailSearchDate, MailSearchExpression,
    MailSearchMailboxSelector, MailSearchPredicate, MailSearchQuery, MailSearchQueryError,
    MailSearchQueryErrorKind, MailSearchText,
};

impl MailSearchQuery {
    pub fn parse(value: impl AsRef<str>) -> Result<Self, MailSearchQueryError> {
        let raw = value.as_ref().trim();
        if raw.is_empty() {
            return Err(MailSearchQueryError::new(
                MailSearchQueryErrorKind::Empty,
                0,
            ));
        }
        let tokens = lex_search_query(raw)?;
        let expression = parse_search_expression(raw, &tokens)?;
        Ok(Self {
            raw: raw.to_string(),
            expression,
        })
    }
}

#[derive(Clone, Copy)]
struct SearchToken<'a> {
    raw: &'a str,
    byte_offset: usize,
}

fn lex_search_query(raw: &str) -> Result<Vec<SearchToken<'_>>, MailSearchQueryError> {
    let mut tokens = Vec::new();
    let mut token_start = None;
    let mut quote_start = None;
    let mut escaped = false;
    for (byte_offset, character) in raw.char_indices() {
        if token_start.is_none() {
            if character.is_whitespace() {
                continue;
            }
            token_start = Some(byte_offset);
        }
        if escaped {
            escaped = false;
            continue;
        }
        if quote_start.is_some() && character == '\\' {
            escaped = true;
            continue;
        }
        if character == '"' {
            quote_start = match quote_start {
                Some(_) => None,
                None => Some(byte_offset),
            };
            continue;
        }
        if character.is_whitespace() && quote_start.is_none() {
            let start = token_start.take().expect("search token start");
            tokens.push(SearchToken {
                raw: &raw[start..byte_offset],
                byte_offset: start,
            });
        }
    }
    if let Some(byte_offset) = quote_start {
        return Err(MailSearchQueryError::new(
            MailSearchQueryErrorKind::UnterminatedQuote,
            byte_offset,
        ));
    }
    if let Some(start) = token_start {
        tokens.push(SearchToken {
            raw: &raw[start..],
            byte_offset: start,
        });
    }
    Ok(tokens)
}

fn parse_search_expression(
    raw: &str,
    tokens: &[SearchToken<'_>],
) -> Result<MailSearchExpression, MailSearchQueryError> {
    let mut disjunction = Vec::new();
    let mut conjunction = Vec::new();
    for token in tokens {
        if token.raw == "OR" {
            if conjunction.is_empty() {
                return Err(MailSearchQueryError::new(
                    MailSearchQueryErrorKind::MissingOperand,
                    token.byte_offset,
                ));
            }
            disjunction.push(compound_expression(conjunction, false));
            conjunction = Vec::new();
            continue;
        }
        conjunction.push(parse_search_token(*token)?);
    }
    if conjunction.is_empty() {
        return Err(MailSearchQueryError::new(
            MailSearchQueryErrorKind::MissingOperand,
            raw.len(),
        ));
    }
    disjunction.push(compound_expression(conjunction, false));
    Ok(compound_expression(disjunction, true))
}

fn compound_expression(
    mut expressions: Vec<MailSearchExpression>,
    disjunction: bool,
) -> MailSearchExpression {
    if expressions.len() == 1 {
        return expressions.pop().expect("single mail search expression");
    }
    if disjunction {
        MailSearchExpression::Any(expressions)
    } else {
        MailSearchExpression::All(expressions)
    }
}

fn parse_search_token(
    token: SearchToken<'_>,
) -> Result<MailSearchExpression, MailSearchQueryError> {
    let (negated, raw) = token
        .raw
        .strip_prefix('-')
        .filter(|raw| !raw.is_empty())
        .map_or((false, token.raw), |raw| (true, raw));
    let expression = MailSearchExpression::Predicate(parse_search_predicate(
        raw,
        token.byte_offset + usize::from(negated),
    )?);
    Ok(if negated {
        MailSearchExpression::Not(Box::new(expression))
    } else {
        expression
    })
}

fn parse_search_predicate(
    raw: &str,
    byte_offset: usize,
) -> Result<MailSearchPredicate, MailSearchQueryError> {
    let Some((operator, value)) = raw.split_once(':') else {
        return Ok(MailSearchPredicate::Text(parse_search_text(
            raw,
            byte_offset,
            None,
        )?));
    };
    let normalized_operator = operator.to_ascii_lowercase();
    match normalized_operator.as_str() {
        "from" => parse_search_text(value, byte_offset + operator.len() + 1, Some(operator))
            .map(MailSearchPredicate::From),
        "to" => parse_search_text(value, byte_offset + operator.len() + 1, Some(operator))
            .map(MailSearchPredicate::To),
        "subject" => parse_search_text(value, byte_offset + operator.len() + 1, Some(operator))
            .map(MailSearchPredicate::Subject),
        "has" if value.eq_ignore_ascii_case("attachment") => Ok(MailSearchPredicate::HasAttachment),
        "in" => parse_mailbox_selector(value, byte_offset + operator.len() + 1, operator)
            .map(MailSearchPredicate::InMailbox),
        "is" if value.eq_ignore_ascii_case("unread") => Ok(MailSearchPredicate::Unread),
        "is" if value.eq_ignore_ascii_case("starred") => Ok(MailSearchPredicate::Starred),
        "is" if value.eq_ignore_ascii_case("shared") => Ok(MailSearchPredicate::Shared),
        "before" => parse_search_date(value, byte_offset + operator.len() + 1, operator)
            .map(MailSearchPredicate::Before),
        "after" => parse_search_date(value, byte_offset + operator.len() + 1, operator)
            .map(MailSearchPredicate::After),
        "older_than" => parse_search_age(value, byte_offset + operator.len() + 1, operator)
            .map(MailSearchPredicate::OlderThan),
        "newer_than" => parse_search_age(value, byte_offset + operator.len() + 1, operator)
            .map(MailSearchPredicate::NewerThan),
        _ => Ok(MailSearchPredicate::Text(parse_search_text(
            raw,
            byte_offset,
            None,
        )?)),
    }
}

fn parse_mailbox_selector(
    raw: &str,
    byte_offset: usize,
    operator: &str,
) -> Result<MailSearchMailboxSelector, MailSearchQueryError> {
    let text = parse_search_text(raw, byte_offset, Some(operator))?;
    Ok(MailSearchMailboxSelector(text.value))
}

fn parse_search_text(
    raw: &str,
    byte_offset: usize,
    operator: Option<&str>,
) -> Result<MailSearchText, MailSearchQueryError> {
    if raw.is_empty() {
        return Err(MailSearchQueryError::new(
            operator.map_or(MailSearchQueryErrorKind::Empty, |operator| {
                MailSearchQueryErrorKind::MissingValue {
                    operator: operator.to_string(),
                }
            }),
            byte_offset,
        ));
    }
    if raw.starts_with('"') && raw.ends_with('"') {
        let value = decode_quoted_search_text(&raw[1..raw.len() - 1]);
        if value.is_empty() {
            return Err(MailSearchQueryError::new(
                operator.map_or(MailSearchQueryErrorKind::Empty, |operator| {
                    MailSearchQueryErrorKind::MissingValue {
                        operator: operator.to_string(),
                    }
                }),
                byte_offset,
            ));
        }
        return Ok(MailSearchText {
            value,
            phrase: true,
        });
    }
    Ok(MailSearchText {
        value: raw.to_string(),
        phrase: false,
    })
}

fn decode_quoted_search_text(raw: &str) -> String {
    let mut decoded = String::with_capacity(raw.len());
    let mut escaped = false;
    for character in raw.chars() {
        if escaped {
            decoded.push(character);
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else {
            decoded.push(character);
        }
    }
    if escaped {
        decoded.push('\\');
    }
    decoded
}

fn parse_search_date(
    raw: &str,
    byte_offset: usize,
    operator: &str,
) -> Result<MailSearchDate, MailSearchQueryError> {
    let invalid = || {
        MailSearchQueryError::new(
            MailSearchQueryErrorKind::InvalidDate {
                operator: operator.to_string(),
                value: raw.to_string(),
            },
            byte_offset,
        )
    };
    let components = raw.split('/').collect::<Vec<_>>();
    if components.len() != 3
        || components[0].len() != 4
        || components[1].len() != 2
        || components[2].len() != 2
    {
        return Err(invalid());
    }
    let year = components[0].parse::<i32>().map_err(|_| invalid())?;
    let month = components[1].parse::<u8>().map_err(|_| invalid())?;
    let day = components[2].parse::<u8>().map_err(|_| invalid())?;
    let max_day = days_in_month(year, month).ok_or_else(invalid)?;
    if !(1..=9999).contains(&year) || day == 0 || day > max_day {
        return Err(invalid());
    }
    Ok(MailSearchDate { year, month, day })
}

fn days_in_month(year: i32, month: u8) -> Option<u8> {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => Some(31),
        4 | 6 | 9 | 11 => Some(30),
        2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => Some(29),
        2 => Some(28),
        _ => None,
    }
}

fn parse_search_age(
    raw: &str,
    byte_offset: usize,
    operator: &str,
) -> Result<MailSearchAge, MailSearchQueryError> {
    let invalid = || {
        MailSearchQueryError::new(
            MailSearchQueryErrorKind::InvalidRelativeAge {
                operator: operator.to_string(),
                value: raw.to_string(),
            },
            byte_offset,
        )
    };
    let (unit_offset, unit) = raw.char_indices().next_back().ok_or_else(invalid)?;
    let amount = &raw[..unit_offset];
    let amount = amount
        .parse::<u32>()
        .ok()
        .and_then(NonZeroU32::new)
        .ok_or_else(invalid)?;
    let unit = match unit {
        'd' => MailSearchAgeUnit::Days,
        'm' => MailSearchAgeUnit::Months,
        'y' => MailSearchAgeUnit::Years,
        _ => return Err(invalid()),
    };
    Ok(MailSearchAge { amount, unit })
}
