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
