use crate::FurnaceIoError;

pub(crate) fn parse_single_column_strings(output: &str) -> Result<Vec<String>, FurnaceIoError> {
    Ok(output
        .lines()
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect())
}

pub(crate) fn first_tsv_value(output: &str) -> Option<String> {
    output.lines().next().map(|line| line.trim().to_string())
}

pub(crate) fn parse_f64(value: &str) -> Result<Option<f64>, FurnaceIoError> {
    if value == "\\N" || value.is_empty() {
        return Ok(None);
    }
    value
        .parse::<f64>()
        .map(Some)
        .map_err(|_| FurnaceIoError::Parse(format!("invalid Float64 value: {value}")))
}

pub(crate) fn parse_u64(value: &str) -> Result<u64, FurnaceIoError> {
    if value.is_empty() || value == "\\N" {
        return Ok(0);
    }
    value
        .parse::<u64>()
        .map_err(|_| FurnaceIoError::Parse(format!("invalid UInt64 value: {value}")))
}

pub(crate) fn symbol_where_clause(symbols: &[String], all_symbols_requested: bool) -> String {
    symbol_where_clause_for_column("security_code", symbols, all_symbols_requested)
}

pub(crate) fn symbol_where_clause_for_column(
    column: &str,
    symbols: &[String],
    all_symbols_requested: bool,
) -> String {
    if all_symbols_requested {
        return "1 = 1".to_string();
    }
    if symbols.is_empty() {
        return "1 = 0".to_string();
    }
    let values = symbols
        .iter()
        .map(|symbol| format!("'{}'", sql_string(symbol)))
        .collect::<Vec<_>>()
        .join(", ");
    format!("{column} IN ({values})")
}

pub(crate) fn symbol_where_clause_for(
    column: &str,
    symbols: &[String],
    all_symbols_requested: bool,
) -> String {
    symbol_where_clause_for_column(column, symbols, all_symbols_requested)
}

pub(crate) fn sql_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('\'', "\\'")
}
