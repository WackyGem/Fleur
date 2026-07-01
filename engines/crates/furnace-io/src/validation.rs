use crate::FurnaceIoError;
use time::{Date, Month};

pub(crate) fn affected_years(from: &str, to: &str) -> Result<Vec<u16>, FurnaceIoError> {
    let from_year = parse_year(from)?;
    let to_year = parse_year(to)?;
    Ok((from_year..=to_year).collect())
}

fn parse_year(date: &str) -> Result<u16, FurnaceIoError> {
    validate_date("date", date)?;
    date[0..4]
        .parse::<u16>()
        .map_err(|_| FurnaceIoError::Parse(format!("invalid date year: {date}")))
}

pub(crate) fn validate_date(name: &str, value: &str) -> Result<(), FurnaceIoError> {
    let bytes = value.as_bytes();
    if bytes.len() != 10
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || !bytes[0..4].iter().all(u8::is_ascii_digit)
        || !bytes[5..7].iter().all(u8::is_ascii_digit)
        || !bytes[8..10].iter().all(u8::is_ascii_digit)
    {
        return Err(FurnaceIoError::InvalidRequest(format!(
            "{name} must use YYYY-MM-DD format"
        )));
    }
    Ok(())
}

pub(crate) fn parse_clickhouse_date(value: &str) -> Result<Date, FurnaceIoError> {
    validate_date("date", value)?;
    let year = value[0..4]
        .parse::<i32>()
        .map_err(|_| FurnaceIoError::DateConversion(format!("invalid date year: {value}")))?;
    let month = value[5..7]
        .parse::<u8>()
        .map_err(|_| FurnaceIoError::DateConversion(format!("invalid date month: {value}")))?;
    let day = value[8..10]
        .parse::<u8>()
        .map_err(|_| FurnaceIoError::DateConversion(format!("invalid date day: {value}")))?;
    Date::from_calendar_date(
        year,
        Month::try_from(month)
            .map_err(|_| FurnaceIoError::DateConversion(format!("invalid date month: {value}")))?,
        day,
    )
    .map_err(|source| FurnaceIoError::DateConversion(format!("invalid date {value}: {source}")))
}

pub(crate) fn format_clickhouse_date(value: Date) -> String {
    format!(
        "{:04}-{:02}-{:02}",
        value.year(),
        u8::from(value.month()),
        value.day()
    )
}

pub(crate) fn validate_table_name(name: &str, value: &str) -> Result<(), FurnaceIoError> {
    let parts = value.split('.').collect::<Vec<_>>();
    if parts.len() != 2 {
        return Err(FurnaceIoError::InvalidRequest(format!(
            "{name} must use database.table format"
        )));
    }
    for part in parts {
        validate_identifier(name, part)?;
    }
    Ok(())
}

pub(crate) fn validate_identifier(name: &str, value: &str) -> Result<(), FurnaceIoError> {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return Err(FurnaceIoError::InvalidRequest(format!(
            "{name} must not be empty"
        )));
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return Err(FurnaceIoError::InvalidRequest(format!(
            "{name} must start with an ASCII letter or underscore"
        )));
    }
    if !chars.all(|character| character.is_ascii_alphanumeric() || character == '_') {
        return Err(FurnaceIoError::InvalidRequest(format!(
            "{name} must contain only ASCII letters, digits, or underscores"
        )));
    }
    Ok(())
}
