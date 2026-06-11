use std::collections::HashMap;
use std::time::Instant;

use furnace_core::{
    MacdInput, MacdOutputValues, MacdPreviousState, calculate_macd_series_from_previous_state,
    visit_macd_series_from_previous_state,
};
use rayon::prelude::*;

use crate::FurnaceIoError;
use crate::request::MacdRunRequest;
use crate::rowbinary::{read_rowbinary_nullable_f64, read_rowbinary_string};
use crate::rows::{
    MacdCalculationResult, MacdGroupedInput, MacdResultRow, MacdSecurityCalculation,
};
use crate::runners::shared::should_parallelize;

#[derive(Debug, Clone, PartialEq)]
pub(super) struct MacdDryRunCalculation {
    pub(super) result: MacdCalculationResult,
    pub(super) input_rows: u64,
    pub(super) valid_close_rows: u64,
    pub(super) input_from: Option<String>,
}

pub(super) fn calculate_macd_dry_run_from_rowbinary(
    request: &MacdRunRequest,
    effective_output_to: &str,
    input_bytes: &[u8],
    states: &HashMap<String, MacdPreviousState>,
) -> Result<MacdDryRunCalculation, FurnaceIoError> {
    let compute_started = Instant::now();
    let mut cursor = 0;
    let mut input_rows = 0;
    let mut valid_close_rows = 0;
    let mut input_from = None::<String>;
    let mut current_security_code = None::<String>;
    let mut current_inputs = Vec::new();
    let mut output_rows = 0;
    let mut null_indicator_rows = 0;

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

        if current_security_code.as_deref() != Some(security_code)
            && let Some(security_code) = current_security_code.replace(security_code.to_string())
        {
            let calculated = calculate_macd_security_counts(
                request,
                effective_output_to,
                states.get(security_code.as_str()).cloned(),
                &MacdGroupedInput {
                    security_code,
                    inputs: std::mem::take(&mut current_inputs),
                },
            )?;
            output_rows += calculated.output_rows;
            null_indicator_rows += calculated.null_indicator_rows;
        }

        current_inputs.push(MacdInput::new(trade_date.to_string(), close_price));
        input_rows += 1;
    }

    if let Some(security_code) = current_security_code {
        let calculated = calculate_macd_security_counts(
            request,
            effective_output_to,
            states.get(security_code.as_str()).cloned(),
            &MacdGroupedInput {
                security_code,
                inputs: current_inputs,
            },
        )?;
        output_rows += calculated.output_rows;
        null_indicator_rows += calculated.null_indicator_rows;
    }

    Ok(MacdDryRunCalculation {
        result: MacdCalculationResult {
            rows: Vec::new(),
            output_rows,
            valid_close_rows,
            null_indicator_rows,
            compute_elapsed: compute_started.elapsed(),
            parallelism: "serial-streaming",
            worker_threads: rayon::current_num_threads(),
        },
        input_rows,
        valid_close_rows,
        input_from,
    })
}

pub(super) fn calculate_macd_outputs(
    request: &MacdRunRequest,
    effective_output_to: &str,
    groups: Vec<MacdGroupedInput>,
    input_row_count: usize,
    states: &HashMap<String, MacdPreviousState>,
    collect_rows: bool,
) -> Result<MacdCalculationResult, FurnaceIoError> {
    let worker_threads = rayon::current_num_threads();
    let parallel = should_parallelize(groups.len(), input_row_count, worker_threads);
    let compute_started = Instant::now();
    let mut calculated = if parallel {
        calculate_macd_grouped_outputs_parallel_with_collection(
            request,
            effective_output_to,
            &groups,
            states,
            collect_rows,
        )?
    } else {
        calculate_macd_grouped_outputs_serial_with_collection(
            request,
            effective_output_to,
            &groups,
            states,
            collect_rows,
        )?
    };
    if collect_rows {
        calculated.rows.sort_by(|left, right| {
            left.security_code
                .cmp(&right.security_code)
                .then(left.trade_date.cmp(&right.trade_date))
        });
    }
    Ok(MacdCalculationResult {
        rows: calculated.rows,
        output_rows: calculated.output_rows,
        valid_close_rows: calculated.valid_close_rows,
        null_indicator_rows: calculated.null_indicator_rows,
        compute_elapsed: compute_started.elapsed(),
        parallelism: if parallel { "rayon" } else { "serial" },
        worker_threads,
    })
}

pub(in crate::runners) fn calculate_macd_grouped_outputs_serial_with_collection(
    request: &MacdRunRequest,
    effective_output_to: &str,
    groups: &[MacdGroupedInput],
    states: &HashMap<String, MacdPreviousState>,
    collect_rows: bool,
) -> Result<MacdSecurityCalculation, FurnaceIoError> {
    let mut output_rows = Vec::new();
    let mut output_row_count = 0;
    let mut valid_close_rows = 0;
    let mut null_indicator_rows = 0;
    for group in groups {
        let calculated = calculate_macd_security_outputs(
            request,
            effective_output_to,
            states,
            group,
            collect_rows,
        )?;
        output_row_count += calculated.output_rows;
        valid_close_rows += calculated.valid_close_rows;
        null_indicator_rows += calculated.null_indicator_rows;
        output_rows.extend(calculated.rows);
    }
    Ok(MacdSecurityCalculation {
        rows: output_rows,
        output_rows: output_row_count,
        valid_close_rows,
        null_indicator_rows,
    })
}

pub(in crate::runners) fn calculate_macd_grouped_outputs_parallel_with_collection(
    request: &MacdRunRequest,
    effective_output_to: &str,
    groups: &[MacdGroupedInput],
    states: &HashMap<String, MacdPreviousState>,
    collect_rows: bool,
) -> Result<MacdSecurityCalculation, FurnaceIoError> {
    let nested = groups
        .par_iter()
        .map(|group| {
            calculate_macd_security_outputs(
                request,
                effective_output_to,
                states,
                group,
                collect_rows,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut rows = Vec::new();
    let mut output_row_count = 0;
    let mut valid_close_rows = 0;
    let mut null_indicator_rows = 0;
    for calculated in nested {
        output_row_count += calculated.output_rows;
        valid_close_rows += calculated.valid_close_rows;
        null_indicator_rows += calculated.null_indicator_rows;
        rows.extend(calculated.rows);
    }
    Ok(MacdSecurityCalculation {
        rows,
        output_rows: output_row_count,
        valid_close_rows,
        null_indicator_rows,
    })
}

fn calculate_macd_security_outputs(
    request: &MacdRunRequest,
    effective_output_to: &str,
    states: &HashMap<String, MacdPreviousState>,
    group: &MacdGroupedInput,
    collect_rows: bool,
) -> Result<MacdSecurityCalculation, FurnaceIoError> {
    let previous_state = states.get(group.security_code.as_str()).cloned();
    if !collect_rows {
        return calculate_macd_security_counts(request, effective_output_to, previous_state, group);
    }

    let outputs =
        calculate_macd_series_from_previous_state(&group.inputs, &request.params, previous_state)
            .map_err(|source| FurnaceIoError::Compute(source.to_string()))?;
    let mut output_rows = Vec::new();
    let mut output_row_count = 0;
    let mut valid_close_rows = 0;
    let mut null_indicator_rows = 0;
    for (input, output) in group.inputs.iter().zip(outputs) {
        if output.trade_date.as_str() < request.request_from.as_str()
            || output.trade_date.as_str() > effective_output_to
        {
            continue;
        }
        output_row_count += 1;
        if input.close_price.is_some() {
            valid_close_rows += 1;
        }
        if output.all_business_indicators_null() {
            null_indicator_rows += 1;
        }
        if collect_rows {
            output_rows.push(MacdResultRow {
                security_code: group.security_code.clone(),
                trade_date: output.trade_date,
                ema_fast_state_12: output.ema_fast_state_12,
                ema_slow_state_26: output.ema_slow_state_26,
                macd_dif: output.macd_dif,
                macd_dea: output.macd_dea,
                macd_dea_state: output.macd_dea_state,
                macd_histogram: output.macd_histogram,
            });
        }
    }
    Ok(MacdSecurityCalculation {
        rows: output_rows,
        output_rows: output_row_count,
        valid_close_rows,
        null_indicator_rows,
    })
}

fn calculate_macd_security_counts(
    request: &MacdRunRequest,
    effective_output_to: &str,
    previous_state: Option<MacdPreviousState>,
    group: &MacdGroupedInput,
) -> Result<MacdSecurityCalculation, FurnaceIoError> {
    let mut output_row_count = 0;
    let mut valid_close_rows = 0;
    let mut null_indicator_rows = 0;
    let mut input_index = 0;
    visit_macd_series_from_previous_state(
        &group.inputs,
        &request.params,
        previous_state,
        |trade_date, output: MacdOutputValues| {
            let input = &group.inputs[input_index];
            input_index += 1;
            if trade_date < request.request_from.as_str() || trade_date > effective_output_to {
                return;
            }
            output_row_count += 1;
            if input.close_price.is_some() {
                valid_close_rows += 1;
            }
            if output.all_business_indicators_null() {
                null_indicator_rows += 1;
            }
        },
    )
    .map_err(|source| FurnaceIoError::Compute(source.to_string()))?;

    Ok(MacdSecurityCalculation {
        rows: Vec::new(),
        output_rows: output_row_count,
        valid_close_rows,
        null_indicator_rows,
    })
}
