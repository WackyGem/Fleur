use std::time::Instant;

use furnace_core::calculate_price_pattern_series;
use rayon::prelude::*;

use crate::FurnaceIoError;
use crate::request::PricePatternRunRequest;
use crate::rows::{
    PricePatternCalculationResult, PricePatternGroupedInput, PricePatternResultRow,
    PricePatternSecurityCalculation,
};
use crate::runners::shared::{is_valid_price_pattern_structure_bar, should_parallelize};

pub(super) fn calculate_price_pattern_outputs(
    request: &PricePatternRunRequest,
    effective_output_to: &str,
    groups: Vec<PricePatternGroupedInput>,
    input_row_count: usize,
    collect_rows: bool,
) -> Result<PricePatternCalculationResult, FurnaceIoError> {
    let worker_threads = rayon::current_num_threads();
    let parallel = should_parallelize(groups.len(), input_row_count, worker_threads);
    let compute_started = Instant::now();
    let mut calculated = if parallel {
        calculate_price_pattern_grouped_outputs_parallel_with_collection(
            request,
            effective_output_to,
            &groups,
            collect_rows,
        )?
    } else {
        calculate_price_pattern_grouped_outputs_serial_with_collection(
            request,
            effective_output_to,
            &groups,
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
    Ok(PricePatternCalculationResult {
        rows: calculated.rows,
        output_rows: calculated.output_rows,
        valid_streak_rows: calculated.valid_streak_rows,
        valid_structure_bar_rows: calculated.valid_structure_bar_rows,
        null_streak_rows: calculated.null_streak_rows,
        null_second_low_rows: calculated.null_second_low_rows,
        compute_elapsed: compute_started.elapsed(),
        parallelism: if parallel { "rayon" } else { "serial" },
        worker_threads,
    })
}

pub(in crate::runners) fn calculate_price_pattern_grouped_outputs_serial_with_collection(
    request: &PricePatternRunRequest,
    effective_output_to: &str,
    groups: &[PricePatternGroupedInput],
    collect_rows: bool,
) -> Result<PricePatternSecurityCalculation, FurnaceIoError> {
    let mut output_rows = Vec::new();
    let mut output_row_count = 0;
    let mut valid_streak_rows = 0;
    let mut valid_structure_bar_rows = 0;
    let mut null_streak_rows = 0;
    let mut null_second_low_rows = 0;
    for group in groups {
        let calculated = calculate_price_pattern_security_outputs(
            request,
            effective_output_to,
            group,
            collect_rows,
        )?;
        output_row_count += calculated.output_rows;
        valid_streak_rows += calculated.valid_streak_rows;
        valid_structure_bar_rows += calculated.valid_structure_bar_rows;
        null_streak_rows += calculated.null_streak_rows;
        null_second_low_rows += calculated.null_second_low_rows;
        output_rows.extend(calculated.rows);
    }
    Ok(PricePatternSecurityCalculation {
        rows: output_rows,
        output_rows: output_row_count,
        valid_streak_rows,
        valid_structure_bar_rows,
        null_streak_rows,
        null_second_low_rows,
    })
}

pub(in crate::runners) fn calculate_price_pattern_grouped_outputs_parallel_with_collection(
    request: &PricePatternRunRequest,
    effective_output_to: &str,
    groups: &[PricePatternGroupedInput],
    collect_rows: bool,
) -> Result<PricePatternSecurityCalculation, FurnaceIoError> {
    let nested = groups
        .par_iter()
        .map(|group| {
            calculate_price_pattern_security_outputs(
                request,
                effective_output_to,
                group,
                collect_rows,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut rows = Vec::new();
    let mut output_row_count = 0;
    let mut valid_streak_rows = 0;
    let mut valid_structure_bar_rows = 0;
    let mut null_streak_rows = 0;
    let mut null_second_low_rows = 0;
    for calculated in nested {
        output_row_count += calculated.output_rows;
        valid_streak_rows += calculated.valid_streak_rows;
        valid_structure_bar_rows += calculated.valid_structure_bar_rows;
        null_streak_rows += calculated.null_streak_rows;
        null_second_low_rows += calculated.null_second_low_rows;
        rows.extend(calculated.rows);
    }
    Ok(PricePatternSecurityCalculation {
        rows,
        output_rows: output_row_count,
        valid_streak_rows,
        valid_structure_bar_rows,
        null_streak_rows,
        null_second_low_rows,
    })
}

pub(super) fn calculate_price_pattern_security_outputs(
    request: &PricePatternRunRequest,
    effective_output_to: &str,
    group: &PricePatternGroupedInput,
    collect_rows: bool,
) -> Result<PricePatternSecurityCalculation, FurnaceIoError> {
    let outputs = calculate_price_pattern_series(&group.inputs, &request.params, None)
        .map_err(|source| FurnaceIoError::Compute(source.to_string()))?;
    let mut output_rows = Vec::new();
    let mut output_row_count = 0;
    let mut valid_streak_rows = 0;
    let mut valid_structure_bar_rows = 0;
    let mut null_streak_rows = 0;
    let mut null_second_low_rows = 0;
    for (input, output) in group.inputs.iter().zip(outputs) {
        if output.trade_date.as_str() < request.request_from.as_str()
            || output.trade_date.as_str() > effective_output_to
        {
            continue;
        }
        output_row_count += 1;
        if input.close_price.is_some() && input.prev_close_price.is_some() {
            valid_streak_rows += 1;
        }
        if is_valid_price_pattern_structure_bar(input.high_price, input.low_price) {
            valid_structure_bar_rows += 1;
        }
        if output.close_direction.is_none() {
            null_streak_rows += 1;
        }
        if output.n_structure_20_second_low_price.is_none()
            || output.n_structure_20_second_low_ratio.is_none()
        {
            null_second_low_rows += 1;
        }
        if collect_rows {
            output_rows.push(PricePatternResultRow {
                security_code: group.security_code.clone(),
                trade_date: output.trade_date,
                close_direction: output.close_direction,
                close_up_streak_days: output.close_up_streak_days,
                close_down_streak_days: output.close_down_streak_days,
                n_structure_20_valid_bars: output.n_structure_20_valid_bars,
                n_structure_20_high_date: output.n_structure_20_high_date,
                n_structure_20_high_price: output.n_structure_20_high_price,
                n_structure_20_low_date: output.n_structure_20_low_date,
                n_structure_20_low_price: output.n_structure_20_low_price,
                n_structure_20_second_low_date: output.n_structure_20_second_low_date,
                n_structure_20_second_low_price: output.n_structure_20_second_low_price,
                n_structure_20_second_low_ratio: output.n_structure_20_second_low_ratio,
                n_structure_20_is_valid: output.n_structure_20_is_valid,
            });
        }
    }
    Ok(PricePatternSecurityCalculation {
        rows: output_rows,
        output_rows: output_row_count,
        valid_streak_rows,
        valid_structure_bar_rows,
        null_streak_rows,
        null_second_low_rows,
    })
}
