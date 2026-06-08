use crate::FurnaceIoError;

pub(crate) fn date_days_since_unix_epoch(value: &str) -> Result<u16, FurnaceIoError> {
    validate_date("date", value)?;
    let year = value[0..4]
        .parse::<i32>()
        .map_err(|_| FurnaceIoError::Parse(format!("invalid date year: {value}")))?;
    let month = value[5..7]
        .parse::<u32>()
        .map_err(|_| FurnaceIoError::Parse(format!("invalid date month: {value}")))?;
    let day = value[8..10]
        .parse::<u32>()
        .map_err(|_| FurnaceIoError::Parse(format!("invalid date day: {value}")))?;
    let days = days_from_civil(year, month, day);
    u16::try_from(days).map_err(|_| FurnaceIoError::Parse(format!("Date out of range: {value}")))
}

fn days_from_civil(year: i32, month: u32, day: u32) -> i32 {
    let year = year - i32::from(month <= 2);
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = year - era * 400;
    let month = month as i32;
    let day_of_year = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day as i32 - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

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
