use std::collections::HashMap;
use std::time::Instant;

use furnace_core::{KdjState, calculate_kdj_series};
use rayon::prelude::*;

use crate::FurnaceIoError;
use crate::request::KdjRunRequest;
use crate::rows::{KdjCalculationResult, KdjGroupedInput, KdjResultRow, KdjSecurityCalculation};
use crate::runners::shared::should_parallelize;
pub(super) fn calculate_outputs(
    request: &KdjRunRequest,
    effective_output_to: &str,
    groups: Vec<KdjGroupedInput>,
    input_row_count: usize,
    states: &HashMap<String, KdjState>,
    collect_rows: bool,
) -> Result<KdjCalculationResult, FurnaceIoError> {
    let worker_threads = rayon::current_num_threads();
    let parallel = should_parallelize(groups.len(), input_row_count, worker_threads);
    let compute_started = Instant::now();
    let mut calculated = if parallel {
        calculate_grouped_outputs_parallel_with_collection(
            request,
            effective_output_to,
            &groups,
            states,
            collect_rows,
        )?
    } else {
        calculate_grouped_outputs_serial_with_collection(
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
    Ok(KdjCalculationResult {
        rows: calculated.rows,
        output_rows: calculated.output_rows,
        null_indicator_rows: calculated.null_indicator_rows,
        compute_elapsed: compute_started.elapsed(),
        parallelism: if parallel { "rayon" } else { "serial" },
        worker_threads,
    })
}
#[cfg(test)]
pub(in crate::runners) fn calculate_grouped_outputs_serial(
    request: &KdjRunRequest,
    effective_output_to: &str,
    groups: &[KdjGroupedInput],
    states: &HashMap<String, KdjState>,
) -> Result<Vec<KdjResultRow>, FurnaceIoError> {
    Ok(calculate_grouped_outputs_serial_with_collection(
        request,
        effective_output_to,
        groups,
        states,
        true,
    )?
    .rows)
}

pub(in crate::runners) fn calculate_grouped_outputs_serial_with_collection(
    request: &KdjRunRequest,
    effective_output_to: &str,
    groups: &[KdjGroupedInput],
    states: &HashMap<String, KdjState>,
    collect_rows: bool,
) -> Result<KdjSecurityCalculation, FurnaceIoError> {
    let mut output_rows = Vec::new();
    let mut output_row_count = 0;
    let mut null_indicator_rows = 0;
    for group in groups {
        let calculated =
            calculate_security_outputs(request, effective_output_to, states, group, collect_rows)?;
        output_row_count += calculated.output_rows;
        null_indicator_rows += calculated.null_indicator_rows;
        output_rows.extend(calculated.rows);
    }
    Ok(KdjSecurityCalculation {
        rows: output_rows,
        output_rows: output_row_count,
        null_indicator_rows,
    })
}

#[cfg(test)]
pub(in crate::runners) fn calculate_grouped_outputs_parallel(
    request: &KdjRunRequest,
    effective_output_to: &str,
    groups: &[KdjGroupedInput],
    states: &HashMap<String, KdjState>,
) -> Result<Vec<KdjResultRow>, FurnaceIoError> {
    Ok(calculate_grouped_outputs_parallel_with_collection(
        request,
        effective_output_to,
        groups,
        states,
        true,
    )?
    .rows)
}

pub(in crate::runners) fn calculate_grouped_outputs_parallel_with_collection(
    request: &KdjRunRequest,
    effective_output_to: &str,
    groups: &[KdjGroupedInput],
    states: &HashMap<String, KdjState>,
    collect_rows: bool,
) -> Result<KdjSecurityCalculation, FurnaceIoError> {
    let nested = groups
        .par_iter()
        .map(|group| {
            calculate_security_outputs(request, effective_output_to, states, group, collect_rows)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut rows = Vec::new();
    let mut output_row_count = 0;
    let mut null_indicator_rows = 0;
    for calculated in nested {
        output_row_count += calculated.output_rows;
        null_indicator_rows += calculated.null_indicator_rows;
        rows.extend(calculated.rows);
    }
    Ok(KdjSecurityCalculation {
        rows,
        output_rows: output_row_count,
        null_indicator_rows,
    })
}

pub(super) fn calculate_security_outputs(
    request: &KdjRunRequest,
    effective_output_to: &str,
    states: &HashMap<String, KdjState>,
    group: &KdjGroupedInput,
    collect_rows: bool,
) -> Result<KdjSecurityCalculation, FurnaceIoError> {
    let previous_state = states.get(group.security_code.as_str()).copied();
    let outputs = calculate_kdj_series(&group.inputs, request.params, previous_state)
        .map_err(|source| FurnaceIoError::Compute(source.to_string()))?;
    let mut output_rows = Vec::new();
    let mut output_row_count = 0;
    let mut null_indicator_rows = 0;
    for output in outputs {
        if output.trade_date.as_str() < request.request_from.as_str()
            || output.trade_date.as_str() > effective_output_to
        {
            continue;
        }
        output_row_count += 1;
        if output.rsv.is_none() && output.k_value.is_none() && output.d_value.is_none() {
            null_indicator_rows += 1;
        }
        if collect_rows {
            output_rows.push(KdjResultRow {
                security_code: group.security_code.clone(),
                trade_date: output.trade_date,
                rsv_window: request.params.rsv_window,
                k_smoothing: request.params.k_smoothing,
                d_smoothing: request.params.d_smoothing,
                rsv: output.rsv,
                k_value: output.k_value,
                d_value: output.d_value,
                j_value: output.j_value,
            });
        }
    }
    Ok(KdjSecurityCalculation {
        rows: output_rows,
        output_rows: output_row_count,
        null_indicator_rows,
    })
}
