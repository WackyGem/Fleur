use std::time::Instant;

use furnace_core::calculate_boll_series;
use rayon::prelude::*;

use crate::FurnaceIoError;
use crate::request::BollRunRequest;
use crate::rows::{
    BollCalculationResult, BollGroupedInput, BollResultRow, BollSecurityCalculation,
};
use crate::runners::shared::should_parallelize;
pub(super) fn calculate_boll_outputs(
    request: &BollRunRequest,
    effective_output_to: &str,
    groups: Vec<BollGroupedInput>,
    input_row_count: usize,
    collect_rows: bool,
) -> Result<BollCalculationResult, FurnaceIoError> {
    let worker_threads = rayon::current_num_threads();
    let parallel = should_parallelize(groups.len(), input_row_count, worker_threads);
    let compute_started = Instant::now();
    let mut calculated = if parallel {
        calculate_boll_grouped_outputs_parallel_with_collection(
            request,
            effective_output_to,
            &groups,
            collect_rows,
        )?
    } else {
        calculate_boll_grouped_outputs_serial_with_collection(
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
    Ok(BollCalculationResult {
        rows: calculated.rows,
        output_rows: calculated.output_rows,
        output_valid_close_rows: calculated.output_valid_close_rows,
        null_indicator_rows: calculated.null_indicator_rows,
        compute_elapsed: compute_started.elapsed(),
        parallelism: if parallel { "rayon" } else { "serial" },
        worker_threads,
    })
}
pub(in crate::runners) fn calculate_boll_grouped_outputs_serial_with_collection(
    request: &BollRunRequest,
    effective_output_to: &str,
    groups: &[BollGroupedInput],
    collect_rows: bool,
) -> Result<BollSecurityCalculation, FurnaceIoError> {
    let mut output_rows = Vec::new();
    let mut output_row_count = 0;
    let mut output_valid_close_rows = 0;
    let mut null_indicator_rows = 0;
    for group in groups {
        let calculated =
            calculate_boll_security_outputs(request, effective_output_to, group, collect_rows)?;
        output_row_count += calculated.output_rows;
        output_valid_close_rows += calculated.output_valid_close_rows;
        null_indicator_rows += calculated.null_indicator_rows;
        output_rows.extend(calculated.rows);
    }
    Ok(BollSecurityCalculation {
        rows: output_rows,
        output_rows: output_row_count,
        output_valid_close_rows,
        null_indicator_rows,
    })
}

pub(in crate::runners) fn calculate_boll_grouped_outputs_parallel_with_collection(
    request: &BollRunRequest,
    effective_output_to: &str,
    groups: &[BollGroupedInput],
    collect_rows: bool,
) -> Result<BollSecurityCalculation, FurnaceIoError> {
    let nested = groups
        .par_iter()
        .map(|group| {
            calculate_boll_security_outputs(request, effective_output_to, group, collect_rows)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut rows = Vec::new();
    let mut output_row_count = 0;
    let mut output_valid_close_rows = 0;
    let mut null_indicator_rows = 0;
    for calculated in nested {
        output_row_count += calculated.output_rows;
        output_valid_close_rows += calculated.output_valid_close_rows;
        null_indicator_rows += calculated.null_indicator_rows;
        rows.extend(calculated.rows);
    }
    Ok(BollSecurityCalculation {
        rows,
        output_rows: output_row_count,
        output_valid_close_rows,
        null_indicator_rows,
    })
}

pub(super) fn calculate_boll_security_outputs(
    request: &BollRunRequest,
    effective_output_to: &str,
    group: &BollGroupedInput,
    collect_rows: bool,
) -> Result<BollSecurityCalculation, FurnaceIoError> {
    let outputs = calculate_boll_series(&group.inputs, &request.params)
        .map_err(|source| FurnaceIoError::Compute(source.to_string()))?;
    let mut output_rows = Vec::new();
    let mut output_row_count = 0;
    let mut output_valid_close_rows = 0;
    let mut null_indicator_rows = 0;
    for (input, output) in group.inputs.iter().zip(outputs) {
        if output.trade_date.as_str() < request.request_from.as_str()
            || output.trade_date.as_str() > effective_output_to
        {
            continue;
        }
        output_row_count += 1;
        if input.close_price.is_some() {
            output_valid_close_rows += 1;
        }
        if output.all_business_indicators_null() {
            null_indicator_rows += 1;
        }
        if collect_rows {
            output_rows.push(BollResultRow {
                security_code: group.security_code.clone(),
                trade_date: output.trade_date,
                boll_mid_10_1p5: output.boll_mid_10_1p5,
                boll_up_10_1p5: output.boll_up_10_1p5,
                boll_dn_10_1p5: output.boll_dn_10_1p5,
                boll_mid_20_2: output.boll_mid_20_2,
                boll_up_20_2: output.boll_up_20_2,
                boll_dn_20_2: output.boll_dn_20_2,
                boll_mid_50_2p5: output.boll_mid_50_2p5,
                boll_up_50_2p5: output.boll_up_50_2p5,
                boll_dn_50_2p5: output.boll_dn_50_2p5,
            });
        }
    }
    Ok(BollSecurityCalculation {
        rows: output_rows,
        output_rows: output_row_count,
        output_valid_close_rows,
        null_indicator_rows,
    })
}
