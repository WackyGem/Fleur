use furnace_core::{BollInput, KdjInput, MaInput, RsiInput};

use crate::FurnaceIoError;
use crate::rowbinary::{read_rowbinary_nullable_f64, read_rowbinary_string};
use crate::rows::{
    BollGroupedInput, BollInputGroups, KdjGroupedInput, KdjInputGroups, MaGroupedInput,
    MaInputGroups, RsiGroupedInput, RsiInputGroups,
};

pub(in crate::runners) fn group_input_rows(
    input_bytes: &[u8],
) -> Result<KdjInputGroups, FurnaceIoError> {
    let mut groups = Vec::new();
    let mut current_security_code = None;
    let mut current_inputs = Vec::new();
    let mut input_rows = 0;
    let mut cursor = 0;

    while cursor < input_bytes.len() {
        let security_code = read_rowbinary_string(input_bytes, &mut cursor)?;
        let trade_date = read_rowbinary_string(input_bytes, &mut cursor)?;
        let high_price = read_rowbinary_nullable_f64(input_bytes, &mut cursor)?;
        let low_price = read_rowbinary_nullable_f64(input_bytes, &mut cursor)?;
        let close_price = read_rowbinary_nullable_f64(input_bytes, &mut cursor)?;

        if current_security_code.as_deref() != Some(security_code) {
            let previous_security_code = current_security_code.replace(security_code.to_string());
            if let Some(security_code) = previous_security_code {
                groups.push(KdjGroupedInput {
                    security_code,
                    inputs: std::mem::take(&mut current_inputs),
                });
            }
        }

        current_inputs.push(KdjInput::new(
            trade_date.to_string(),
            high_price,
            low_price,
            close_price,
        ));
        input_rows += 1;
    }

    if let Some(security_code) = current_security_code {
        groups.push(KdjGroupedInput {
            security_code,
            inputs: current_inputs,
        });
    }

    Ok(KdjInputGroups { groups, input_rows })
}

pub(in crate::runners) fn group_ma_input_rows(
    input_bytes: &[u8],
) -> Result<MaInputGroups, FurnaceIoError> {
    let mut groups = Vec::new();
    let mut current_security_code = None;
    let mut current_inputs = Vec::new();
    let mut input_rows = 0;
    let mut valid_close_rows = 0;
    let mut valid_volume_rows = 0;
    let mut cursor = 0;

    while cursor < input_bytes.len() {
        let security_code = read_rowbinary_string(input_bytes, &mut cursor)?;
        let trade_date = read_rowbinary_string(input_bytes, &mut cursor)?;
        let close_price = read_rowbinary_nullable_f64(input_bytes, &mut cursor)?;
        let volume = read_rowbinary_nullable_f64(input_bytes, &mut cursor)?;
        if close_price.is_some() {
            valid_close_rows += 1;
        }
        if volume.is_some() {
            valid_volume_rows += 1;
        }

        if current_security_code.as_deref() != Some(security_code) {
            let previous_security_code = current_security_code.replace(security_code.to_string());
            if let Some(security_code) = previous_security_code {
                groups.push(MaGroupedInput {
                    security_code,
                    inputs: std::mem::take(&mut current_inputs),
                });
            }
        }

        current_inputs.push(MaInput::new(trade_date.to_string(), close_price, volume));
        input_rows += 1;
    }

    if let Some(security_code) = current_security_code {
        groups.push(MaGroupedInput {
            security_code,
            inputs: current_inputs,
        });
    }

    Ok(MaInputGroups {
        groups,
        input_rows,
        valid_close_rows,
        valid_volume_rows,
    })
}

pub(in crate::runners) fn group_rsi_input_rows(
    input_bytes: &[u8],
) -> Result<RsiInputGroups, FurnaceIoError> {
    let mut groups = Vec::new();
    let mut current_security_code = None;
    let mut current_inputs = Vec::new();
    let mut input_rows = 0;
    let mut valid_close_rows = 0;
    let mut input_from = None::<String>;
    let mut cursor = 0;

    while cursor < input_bytes.len() {
        let security_code = read_rowbinary_string(input_bytes, &mut cursor)?;
        let trade_date = read_rowbinary_string(input_bytes, &mut cursor)?;
        let close_price = read_rowbinary_nullable_f64(input_bytes, &mut cursor)?;
        input_from = match input_from {
            Some(current) if current.as_str() <= trade_date => Some(current),
            _ => Some(trade_date.to_string()),
        };
        if close_price.is_some() {
            valid_close_rows += 1;
        }

        if current_security_code.as_deref() != Some(security_code) {
            let previous_security_code = current_security_code.replace(security_code.to_string());
            if let Some(security_code) = previous_security_code {
                groups.push(RsiGroupedInput {
                    security_code,
                    inputs: std::mem::take(&mut current_inputs),
                });
            }
        }

        current_inputs.push(RsiInput::new(trade_date.to_string(), close_price));
        input_rows += 1;
    }

    if let Some(security_code) = current_security_code {
        groups.push(RsiGroupedInput {
            security_code,
            inputs: current_inputs,
        });
    }

    Ok(RsiInputGroups {
        groups,
        input_rows,
        valid_close_rows,
        input_from,
    })
}

pub(in crate::runners) fn group_boll_input_rows(
    input_bytes: &[u8],
) -> Result<BollInputGroups, FurnaceIoError> {
    let mut groups = Vec::new();
    let mut current_security_code = None;
    let mut current_inputs = Vec::new();
    let mut input_rows = 0;
    let mut input_valid_close_rows = 0;
    let mut cursor = 0;

    while cursor < input_bytes.len() {
        let security_code = read_rowbinary_string(input_bytes, &mut cursor)?;
        let trade_date = read_rowbinary_string(input_bytes, &mut cursor)?;
        let close_price = read_rowbinary_nullable_f64(input_bytes, &mut cursor)?;
        if close_price.is_some() {
            input_valid_close_rows += 1;
        }

        if current_security_code.as_deref() != Some(security_code) {
            let previous_security_code = current_security_code.replace(security_code.to_string());
            if let Some(security_code) = previous_security_code {
                groups.push(BollGroupedInput {
                    security_code,
                    inputs: std::mem::take(&mut current_inputs),
                });
            }
        }

        current_inputs.push(BollInput::new(trade_date.to_string(), close_price));
        input_rows += 1;
    }

    if let Some(security_code) = current_security_code {
        groups.push(BollGroupedInput {
            security_code,
            inputs: current_inputs,
        });
    }

    Ok(BollInputGroups {
        groups,
        input_rows,
        input_valid_close_rows,
    })
}
