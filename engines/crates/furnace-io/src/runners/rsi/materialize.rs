use std::collections::HashMap;
use std::time::Instant;

use furnace_core::{RsiPreviousState, calculate_rsi_series_from_previous_state};
use rayon::prelude::*;

use crate::FurnaceIoError;
use crate::request::RsiRunRequest;
use crate::rows::{RsiCalculationResult, RsiGroupedInput, RsiResultRow, RsiSecurityCalculation};
use crate::runners::shared::should_parallelize;
pub(super) fn calculate_rsi_outputs(
    request: &RsiRunRequest,
    effective_output_to: &str,
    groups: Vec<RsiGroupedInput>,
    input_row_count: usize,
    states: &HashMap<String, RsiPreviousState>,
    collect_rows: bool,
) -> Result<RsiCalculationResult, FurnaceIoError> {
    let worker_threads = rayon::current_num_threads();
    let parallel = should_parallelize(groups.len(), input_row_count, worker_threads);
    let compute_started = Instant::now();
    let mut calculated = if parallel {
        calculate_rsi_grouped_outputs_parallel_with_collection(
            request,
            effective_output_to,
            &groups,
            states,
            collect_rows,
        )?
    } else {
        calculate_rsi_grouped_outputs_serial_with_collection(
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
    Ok(RsiCalculationResult {
        rows: calculated.rows,
        output_rows: calculated.output_rows,
        valid_close_rows: calculated.valid_close_rows,
        null_indicator_rows: calculated.null_indicator_rows,
        compute_elapsed: compute_started.elapsed(),
        parallelism: if parallel { "rayon" } else { "serial" },
        worker_threads,
    })
}
pub(in crate::runners) fn calculate_rsi_grouped_outputs_serial_with_collection(
    request: &RsiRunRequest,
    effective_output_to: &str,
    groups: &[RsiGroupedInput],
    states: &HashMap<String, RsiPreviousState>,
    collect_rows: bool,
) -> Result<RsiSecurityCalculation, FurnaceIoError> {
    let mut output_rows = Vec::new();
    let mut output_row_count = 0;
    let mut valid_close_rows = 0;
    let mut null_indicator_rows = 0;
    for group in groups {
        let calculated = calculate_rsi_security_outputs(
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
    Ok(RsiSecurityCalculation {
        rows: output_rows,
        output_rows: output_row_count,
        valid_close_rows,
        null_indicator_rows,
    })
}

pub(in crate::runners) fn calculate_rsi_grouped_outputs_parallel_with_collection(
    request: &RsiRunRequest,
    effective_output_to: &str,
    groups: &[RsiGroupedInput],
    states: &HashMap<String, RsiPreviousState>,
    collect_rows: bool,
) -> Result<RsiSecurityCalculation, FurnaceIoError> {
    let nested = groups
        .par_iter()
        .map(|group| {
            calculate_rsi_security_outputs(
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
    Ok(RsiSecurityCalculation {
        rows,
        output_rows: output_row_count,
        valid_close_rows,
        null_indicator_rows,
    })
}

pub(super) fn calculate_rsi_security_outputs(
    request: &RsiRunRequest,
    effective_output_to: &str,
    states: &HashMap<String, RsiPreviousState>,
    group: &RsiGroupedInput,
    collect_rows: bool,
) -> Result<RsiSecurityCalculation, FurnaceIoError> {
    let previous_state = states.get(group.security_code.as_str()).cloned();
    let outputs =
        calculate_rsi_series_from_previous_state(&group.inputs, &request.params, previous_state)
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
            output_rows.push(RsiResultRow {
                security_code: group.security_code.clone(),
                trade_date: output.trade_date,
                rsi_6: output.rsi_6,
                rsi_12: output.rsi_12,
                rsi_14: output.rsi_14,
                rsi_24: output.rsi_24,
                rsi_25: output.rsi_25,
                rsi_50: output.rsi_50,
                avg_gain_6_state: output.avg_gain_6_state,
                avg_loss_6_state: output.avg_loss_6_state,
                avg_gain_12_state: output.avg_gain_12_state,
                avg_loss_12_state: output.avg_loss_12_state,
                avg_gain_14_state: output.avg_gain_14_state,
                avg_loss_14_state: output.avg_loss_14_state,
                avg_gain_24_state: output.avg_gain_24_state,
                avg_loss_24_state: output.avg_loss_24_state,
                avg_gain_25_state: output.avg_gain_25_state,
                avg_loss_25_state: output.avg_loss_25_state,
                avg_gain_50_state: output.avg_gain_50_state,
                avg_loss_50_state: output.avg_loss_50_state,
            });
        }
    }
    Ok(RsiSecurityCalculation {
        rows: output_rows,
        output_rows: output_row_count,
        valid_close_rows,
        null_indicator_rows,
    })
}
