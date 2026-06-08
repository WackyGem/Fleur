use std::collections::HashMap;
use std::time::Instant;

use furnace_core::{MaPreviousState, calculate_ma_series_from_previous_state};
use rayon::prelude::*;

use crate::FurnaceIoError;
use crate::request::MaRunRequest;
use crate::rows::{MaCalculationResult, MaGroupedInput, MaResultRow, MaSecurityCalculation};
use crate::runners::shared::should_parallelize;
pub(super) fn calculate_ma_outputs(
    request: &MaRunRequest,
    effective_output_to: &str,
    groups: Vec<MaGroupedInput>,
    input_row_count: usize,
    states: &HashMap<String, MaPreviousState>,
    collect_rows: bool,
) -> Result<MaCalculationResult, FurnaceIoError> {
    let worker_threads = rayon::current_num_threads();
    let parallel = should_parallelize(groups.len(), input_row_count, worker_threads);
    let compute_started = Instant::now();
    let mut calculated = if parallel {
        calculate_ma_grouped_outputs_parallel_with_collection(
            request,
            effective_output_to,
            &groups,
            states,
            collect_rows,
        )?
    } else {
        calculate_ma_grouped_outputs_serial_with_collection(
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
    Ok(MaCalculationResult {
        rows: calculated.rows,
        output_rows: calculated.output_rows,
        valid_close_rows: calculated.valid_close_rows,
        valid_volume_rows: calculated.valid_volume_rows,
        null_indicator_rows: calculated.null_indicator_rows,
        compute_elapsed: compute_started.elapsed(),
        parallelism: if parallel { "rayon" } else { "serial" },
        worker_threads,
    })
}
pub(in crate::runners) fn calculate_ma_grouped_outputs_serial_with_collection(
    request: &MaRunRequest,
    effective_output_to: &str,
    groups: &[MaGroupedInput],
    states: &HashMap<String, MaPreviousState>,
    collect_rows: bool,
) -> Result<MaSecurityCalculation, FurnaceIoError> {
    let mut output_rows = Vec::new();
    let mut output_row_count = 0;
    let mut valid_close_rows = 0;
    let mut valid_volume_rows = 0;
    let mut null_indicator_rows = 0;
    for group in groups {
        let calculated = calculate_ma_security_outputs(
            request,
            effective_output_to,
            states,
            group,
            collect_rows,
        )?;
        output_row_count += calculated.output_rows;
        valid_close_rows += calculated.valid_close_rows;
        valid_volume_rows += calculated.valid_volume_rows;
        null_indicator_rows += calculated.null_indicator_rows;
        output_rows.extend(calculated.rows);
    }
    Ok(MaSecurityCalculation {
        rows: output_rows,
        output_rows: output_row_count,
        valid_close_rows,
        valid_volume_rows,
        null_indicator_rows,
    })
}

pub(in crate::runners) fn calculate_ma_grouped_outputs_parallel_with_collection(
    request: &MaRunRequest,
    effective_output_to: &str,
    groups: &[MaGroupedInput],
    states: &HashMap<String, MaPreviousState>,
    collect_rows: bool,
) -> Result<MaSecurityCalculation, FurnaceIoError> {
    let nested = groups
        .par_iter()
        .map(|group| {
            calculate_ma_security_outputs(request, effective_output_to, states, group, collect_rows)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut rows = Vec::new();
    let mut output_row_count = 0;
    let mut valid_close_rows = 0;
    let mut valid_volume_rows = 0;
    let mut null_indicator_rows = 0;
    for calculated in nested {
        output_row_count += calculated.output_rows;
        valid_close_rows += calculated.valid_close_rows;
        valid_volume_rows += calculated.valid_volume_rows;
        null_indicator_rows += calculated.null_indicator_rows;
        rows.extend(calculated.rows);
    }
    Ok(MaSecurityCalculation {
        rows,
        output_rows: output_row_count,
        valid_close_rows,
        valid_volume_rows,
        null_indicator_rows,
    })
}
pub(super) fn calculate_ma_security_outputs(
    request: &MaRunRequest,
    effective_output_to: &str,
    states: &HashMap<String, MaPreviousState>,
    group: &MaGroupedInput,
    collect_rows: bool,
) -> Result<MaSecurityCalculation, FurnaceIoError> {
    let previous_state = states.get(group.security_code.as_str()).cloned();
    let outputs =
        calculate_ma_series_from_previous_state(&group.inputs, &request.params, previous_state)
            .map_err(|source| FurnaceIoError::Compute(source.to_string()))?;
    let mut output_rows = Vec::new();
    let mut output_row_count = 0;
    let mut valid_close_rows = 0;
    let mut valid_volume_rows = 0;
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
        if input.volume.is_some() {
            valid_volume_rows += 1;
        }
        if output.all_business_indicators_null() {
            null_indicator_rows += 1;
        }
        if collect_rows {
            output_rows.push(MaResultRow {
                security_code: group.security_code.clone(),
                price_ma_3: output.price_ma(3),
                price_ma_5: output.price_ma(5),
                price_ma_6: output.price_ma(6),
                price_ma_10: output.price_ma(10),
                price_ma_12: output.price_ma(12),
                price_ma_14: output.price_ma(14),
                price_ma_20: output.price_ma(20),
                price_ma_24: output.price_ma(24),
                price_ma_28: output.price_ma(28),
                price_ma_57: output.price_ma(57),
                price_ma_60: output.price_ma(60),
                price_ma_114: output.price_ma(114),
                price_ma_250: output.price_ma(250),
                price_avg_ma_3_6_12_24: output.price_avg_ma_3_6_12_24,
                price_avg_ma_14_28_57_114: output.price_avg_ma_14_28_57_114,
                price_ema1_10_state: output.price_ema1_10_state,
                price_ema2_10: output.price_ema2_10,
                price_ema2_10_state: output.price_ema2_10_state,
                volume_ma_5: output.volume_ma(5),
                volume_ma_10: output.volume_ma(10),
                volume_ma_20: output.volume_ma(20),
                volume_ma_60: output.volume_ma(60),
                trade_date: output.trade_date,
            });
        }
    }
    Ok(MaSecurityCalculation {
        rows: output_rows,
        output_rows: output_row_count,
        valid_close_rows,
        valid_volume_rows,
        null_indicator_rows,
    })
}
