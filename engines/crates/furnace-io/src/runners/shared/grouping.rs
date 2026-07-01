use furnace_core::{BollInput, KdjInput, MaInput, MacdInput, PricePatternInput, RsiInput};

use crate::rows::{
    BollGroupedInput, BollInputGroups, CloseInputRow, KdjGroupedInput, KdjInputGroups, KdjInputRow,
    MaGroupedInput, MaInputGroups, MaInputRow, MacdGroupedInput, MacdInputGroups,
    PricePatternGroupedInput, PricePatternInputGroups, PricePatternInputRow, RsiGroupedInput,
    RsiInputGroups,
};
use crate::validation::format_clickhouse_date;

pub(in crate::runners) fn group_input_rows(rows: Vec<KdjInputRow>) -> KdjInputGroups {
    let mut groups = Vec::new();
    let mut current_security_code = None;
    let mut current_inputs = Vec::new();
    let input_rows = rows.len() as u64;

    for row in rows {
        if current_security_code.as_deref() != Some(row.security_code.as_str()) {
            let previous_security_code =
                current_security_code.replace(row.security_code.to_string());
            if let Some(security_code) = previous_security_code {
                groups.push(KdjGroupedInput {
                    security_code,
                    inputs: std::mem::take(&mut current_inputs),
                });
            }
        }

        current_inputs.push(KdjInput::new(
            format_clickhouse_date(row.trade_date),
            row.high_price,
            row.low_price,
            row.close_price,
        ));
    }

    if let Some(security_code) = current_security_code {
        groups.push(KdjGroupedInput {
            security_code,
            inputs: current_inputs,
        });
    }

    KdjInputGroups { groups, input_rows }
}

pub(in crate::runners) fn group_ma_input_rows(rows: Vec<MaInputRow>) -> MaInputGroups {
    let mut groups = Vec::new();
    let mut current_security_code = None;
    let mut current_inputs = Vec::new();
    let mut valid_close_rows = 0;
    let mut valid_volume_rows = 0;
    let input_rows = rows.len() as u64;

    for row in rows {
        if row.close_price.is_some() {
            valid_close_rows += 1;
        }
        if row.volume.is_some() {
            valid_volume_rows += 1;
        }

        if current_security_code.as_deref() != Some(row.security_code.as_str()) {
            let previous_security_code =
                current_security_code.replace(row.security_code.to_string());
            if let Some(security_code) = previous_security_code {
                groups.push(MaGroupedInput {
                    security_code,
                    inputs: std::mem::take(&mut current_inputs),
                });
            }
        }

        current_inputs.push(MaInput::new(
            format_clickhouse_date(row.trade_date),
            row.close_price,
            row.volume,
        ));
    }

    if let Some(security_code) = current_security_code {
        groups.push(MaGroupedInput {
            security_code,
            inputs: current_inputs,
        });
    }

    MaInputGroups {
        groups,
        input_rows,
        valid_close_rows,
        valid_volume_rows,
    }
}

pub(in crate::runners) fn group_rsi_input_rows(rows: Vec<CloseInputRow>) -> RsiInputGroups {
    let mut groups = Vec::new();
    let mut current_security_code = None;
    let mut current_inputs = Vec::new();
    let mut valid_close_rows = 0;
    let mut input_from = None::<String>;
    let input_rows = rows.len() as u64;

    for row in rows {
        let trade_date = format_clickhouse_date(row.trade_date);
        input_from = match input_from {
            Some(current) if current.as_str() <= trade_date.as_str() => Some(current),
            _ => Some(trade_date.clone()),
        };
        if row.close_price.is_some() {
            valid_close_rows += 1;
        }

        if current_security_code.as_deref() != Some(row.security_code.as_str()) {
            let previous_security_code =
                current_security_code.replace(row.security_code.to_string());
            if let Some(security_code) = previous_security_code {
                groups.push(RsiGroupedInput {
                    security_code,
                    inputs: std::mem::take(&mut current_inputs),
                });
            }
        }

        current_inputs.push(RsiInput::new(trade_date, row.close_price));
    }

    if let Some(security_code) = current_security_code {
        groups.push(RsiGroupedInput {
            security_code,
            inputs: current_inputs,
        });
    }

    RsiInputGroups {
        groups,
        input_rows,
        valid_close_rows,
        input_from,
    }
}

pub(in crate::runners) fn group_boll_input_rows(rows: Vec<CloseInputRow>) -> BollInputGroups {
    let mut groups = Vec::new();
    let mut current_security_code = None;
    let mut current_inputs = Vec::new();
    let mut input_valid_close_rows = 0;
    let input_rows = rows.len() as u64;

    for row in rows {
        if row.close_price.is_some() {
            input_valid_close_rows += 1;
        }

        if current_security_code.as_deref() != Some(row.security_code.as_str()) {
            let previous_security_code =
                current_security_code.replace(row.security_code.to_string());
            if let Some(security_code) = previous_security_code {
                groups.push(BollGroupedInput {
                    security_code,
                    inputs: std::mem::take(&mut current_inputs),
                });
            }
        }

        current_inputs.push(BollInput::new(
            format_clickhouse_date(row.trade_date),
            row.close_price,
        ));
    }

    if let Some(security_code) = current_security_code {
        groups.push(BollGroupedInput {
            security_code,
            inputs: current_inputs,
        });
    }

    BollInputGroups {
        groups,
        input_rows,
        input_valid_close_rows,
    }
}

pub(in crate::runners) fn group_macd_input_rows(rows: Vec<CloseInputRow>) -> MacdInputGroups {
    let mut groups = Vec::new();
    let mut current_security_code = None;
    let mut current_inputs = Vec::new();
    let mut valid_close_rows = 0;
    let mut input_from = None::<String>;
    let input_rows = rows.len() as u64;

    for row in rows {
        let trade_date = format_clickhouse_date(row.trade_date);
        input_from = match input_from {
            Some(current) if current.as_str() <= trade_date.as_str() => Some(current),
            _ => Some(trade_date.clone()),
        };
        if row.close_price.is_some() {
            valid_close_rows += 1;
        }

        if current_security_code.as_deref() != Some(row.security_code.as_str()) {
            let previous_security_code =
                current_security_code.replace(row.security_code.to_string());
            if let Some(security_code) = previous_security_code {
                groups.push(MacdGroupedInput {
                    security_code,
                    inputs: std::mem::take(&mut current_inputs),
                });
            }
        }

        current_inputs.push(MacdInput::new(trade_date, row.close_price));
    }

    if let Some(security_code) = current_security_code {
        groups.push(MacdGroupedInput {
            security_code,
            inputs: current_inputs,
        });
    }

    MacdInputGroups {
        groups,
        input_rows,
        valid_close_rows,
        input_from,
    }
}

pub(in crate::runners) fn group_price_pattern_input_rows(
    rows: Vec<PricePatternInputRow>,
) -> PricePatternInputGroups {
    let mut groups = Vec::new();
    let mut current_security_code = None;
    let mut current_inputs = Vec::new();
    let mut input_valid_streak_rows = 0;
    let mut input_valid_structure_bar_rows = 0;
    let input_rows = rows.len() as u64;

    for row in rows {
        if row.close_price.is_some() && row.prev_close_price.is_some() {
            input_valid_streak_rows += 1;
        }
        if is_valid_price_pattern_structure_bar(row.high_price, row.low_price) {
            input_valid_structure_bar_rows += 1;
        }

        if current_security_code.as_deref() != Some(row.security_code.as_str()) {
            let previous_security_code =
                current_security_code.replace(row.security_code.to_string());
            if let Some(security_code) = previous_security_code {
                groups.push(PricePatternGroupedInput {
                    security_code,
                    inputs: std::mem::take(&mut current_inputs),
                });
            }
        }

        current_inputs.push(PricePatternInput::new(
            format_clickhouse_date(row.trade_date),
            row.high_price,
            row.low_price,
            row.close_price,
            row.prev_close_price,
        ));
    }

    if let Some(security_code) = current_security_code {
        groups.push(PricePatternGroupedInput {
            security_code,
            inputs: current_inputs,
        });
    }

    PricePatternInputGroups {
        groups,
        input_rows,
        input_valid_streak_rows,
        input_valid_structure_bar_rows,
    }
}

pub(in crate::runners) fn is_valid_price_pattern_structure_bar(
    high_price: Option<f64>,
    low_price: Option<f64>,
) -> bool {
    matches!(
        (high_price, low_price),
        (Some(high), Some(low)) if high.is_finite() && low.is_finite() && low > 0.0 && high >= low
    )
}
