use super::*;
#[test]
fn run_kdj_dry_run_reads_inputs_and_computes_summary() {
    let responses = ["sh.600000\n", "2026-01-01\n", "1\n", ""];
    let input_rows = rowbinary_input_rows(&[
        ("sh.600000", "2026-01-01", Some(10.0), Some(8.0), Some(9.0)),
        ("sh.600000", "2026-01-02", Some(11.0), Some(8.0), Some(10.0)),
        ("sh.600000", "2026-01-03", Some(12.0), Some(8.0), Some(11.0)),
    ]);
    let mut executor = FakeExecutor::with_responses_and_bytes(&responses, vec![input_rows]);
    let request = KdjRunRequest {
        request_from: "2026-01-01".to_string(),
        request_to: "2026-01-03".to_string(),
        params: KdjParams {
            rsv_window: 3,
            ..KdjParams::default()
        },
        ..KdjRunRequest::default()
    };

    let summary = run_kdj(&mut executor, &request).unwrap();

    assert_eq!(summary.input_rows, 3);
    assert_eq!(summary.output_rows, 3);
    assert_eq!(summary.null_indicator_rows, 2);
    assert!(summary.performance_metrics.input_rows_per_sec.is_finite());
    assert!(summary.to_json().contains("\"performance_metrics\""));
    assert!(executor.queries.iter().any(|query| {
        query.contains("AND 1 = 1\nORDER BY security_code, trade_date\nFORMAT RowBinary")
    }));
    assert!(!summary.writes_applied);
}
#[test]
fn parallel_kdj_outputs_match_serial_outputs() {
    let request = KdjRunRequest {
        request_from: "2026-01-01".to_string(),
        request_to: "2026-01-04".to_string(),
        params: KdjParams {
            rsv_window: 3,
            ..KdjParams::default()
        },
        ..KdjRunRequest::default()
    };
    let groups = vec![
        KdjGroupedInput {
            security_code: "sh.600000".to_string(),
            inputs: vec![
                KdjInput::new("2026-01-01".to_string(), Some(10.0), Some(8.0), Some(9.0)),
                KdjInput::new("2026-01-02".to_string(), Some(11.0), Some(8.0), Some(10.0)),
                KdjInput::new("2026-01-03".to_string(), Some(12.0), Some(8.0), Some(11.0)),
                KdjInput::new("2026-01-04".to_string(), Some(13.0), Some(8.0), Some(12.0)),
            ],
        },
        KdjGroupedInput {
            security_code: "sz.000001".to_string(),
            inputs: vec![
                KdjInput::new("2026-01-01".to_string(), Some(20.0), Some(18.0), Some(19.0)),
                KdjInput::new("2026-01-02".to_string(), Some(21.0), Some(18.0), Some(20.0)),
                KdjInput::new("2026-01-03".to_string(), Some(22.0), Some(18.0), Some(21.0)),
                KdjInput::new("2026-01-04".to_string(), Some(23.0), Some(18.0), Some(22.0)),
            ],
        },
    ];
    let states = HashMap::from([("sz.000001".to_string(), KdjState::new(52.0, 48.0))]);

    let mut serial =
        calculate_grouped_outputs_serial(&request, "2026-01-04", &groups, &states).unwrap();
    let mut parallel =
        calculate_grouped_outputs_parallel(&request, "2026-01-04", &groups, &states).unwrap();
    serial.sort_by(|left, right| {
        left.security_code
            .cmp(&right.security_code)
            .then(left.trade_date.cmp(&right.trade_date))
    });
    parallel.sort_by(|left, right| {
        left.security_code
            .cmp(&right.security_code)
            .then(left.trade_date.cmp(&right.trade_date))
    });

    assert_eq!(parallel, serial);
}

#[test]
fn run_kdj_append_latest_inserts_result_rows() {
    let responses = ["2026-01-01\n", "1\n", "", "0\n"];
    let input_rows = rowbinary_input_rows(&[
        ("sh.600000", "2026-01-01", Some(10.0), Some(8.0), Some(9.0)),
        ("sh.600000", "2026-01-02", Some(11.0), Some(8.0), Some(10.0)),
        ("sh.600000", "2026-01-03", Some(12.0), Some(8.0), Some(11.0)),
    ]);
    let mut executor = FakeExecutor::with_responses_and_bytes(&responses, vec![input_rows]);
    let request = KdjRunRequest {
        request_from: "2026-01-01".to_string(),
        request_to: "2026-01-03".to_string(),
        symbols: vec!["sh.600000".to_string()],
        mode: KdjWriteMode::AppendLatest,
        insert_batch_size: MIN_INSERT_BATCH_SIZE,
        ..KdjRunRequest::default()
    };

    let summary = run_kdj(&mut executor, &request).unwrap();

    assert!(summary.writes_applied);
    assert_eq!(executor.byte_inserts.len(), 1);
    assert!(executor.byte_inserts[0].0.contains("FORMAT RowBinary"));
    assert!(executor.byte_inserts[0].1.starts_with(b"\tsh.600000"));
}

#[test]
fn kdj_result_row_writes_clickhouse_rowbinary_encoding() {
    let row = KdjResultRow {
        security_code: "sh.600000".to_string(),
        trade_date: "2026-01-03".to_string(),
        rsv_window: 9,
        k_smoothing: 3,
        d_smoothing: 3,
        rsv: None,
        k_value: Some(12.5),
        d_value: None,
        j_value: Some(1.25),
    };
    let mut bytes = Vec::new();

    row.write_row_binary(&mut bytes).unwrap();

    let mut expected = Vec::new();
    expected.push(9);
    expected.extend_from_slice(b"sh.600000");
    expected.extend_from_slice(&20_456_u16.to_le_bytes());
    expected.extend_from_slice(&9_u16.to_le_bytes());
    expected.extend_from_slice(&3_u16.to_le_bytes());
    expected.extend_from_slice(&3_u16.to_le_bytes());
    expected.push(1);
    expected.push(0);
    expected.extend_from_slice(&12.5_f64.to_le_bytes());
    expected.push(1);
    expected.push(0);
    expected.extend_from_slice(&1.25_f64.to_le_bytes());
    assert_eq!(bytes, expected);
}

#[test]
fn retain_old_rows_skips_fully_covered_all_market_year_partitions() {
    let mut executor = FakeExecutor::default();
    let request = KdjRunRequest {
        request_from: "2020-01-01".to_string(),
        request_to: "2022-12-31".to_string(),
        mode: KdjWriteMode::ReplaceCascade,
        ..KdjRunRequest::default()
    };

    let retained = retain_existing_rows_for_staging(
        &mut executor,
        &RetainStagingRows {
            output_table: DEFAULT_KDJ_OUTPUT_TABLE,
            staging_table: "fleur_calculation.stage",
            request_from: &request.request_from,
            symbols: &[],
            all_symbols_requested: true,
            years: &[2020, 2021, 2022],
            effective_output_to: "2022-12-31",
        },
    )
    .unwrap();

    assert_eq!(retained, 0);
    assert!(executor.queries.is_empty());
}

#[test]
fn validate_staging_checks_all_years_with_one_query() {
    let mut executor = FakeExecutor::with_responses_and_bytes(&["0\n"], Vec::new());

    let summary = validate_staging(
        &mut executor,
        "fleur_calculation.stage",
        &[2020, 2021, 2022],
    )
    .unwrap();

    assert_eq!(summary, ValidationSummary::passed());
    assert_eq!(executor.queries.len(), 1);
    assert!(executor.queries[0].contains("toYear(trade_date) IN (2020,2021,2022)"));
}

#[test]
fn run_kdj_replace_cascade_batches_partition_replace_statements() {
    let responses = [
        "2027-01-02\n",
        "2026-12-30\n",
        "1\n",
        "",
        "0\n",
        "0\n",
        "0\n",
    ];
    let input_rows = rowbinary_input_rows(&[
        ("sh.600000", "2026-12-30", Some(10.0), Some(8.0), Some(9.0)),
        ("sh.600000", "2026-12-31", Some(11.0), Some(8.0), Some(10.0)),
        ("sh.600000", "2027-01-01", Some(12.0), Some(8.0), Some(11.0)),
        ("sh.600000", "2027-01-02", Some(13.0), Some(8.0), Some(12.0)),
    ]);
    let mut executor = FakeExecutor::with_responses_and_bytes(&responses, vec![input_rows]);
    let request = KdjRunRequest {
        request_from: "2026-12-30".to_string(),
        request_to: "2026-12-31".to_string(),
        symbols: vec!["sh.600000".to_string()],
        run_id: Some("replace-batch-test".to_string()),
        mode: KdjWriteMode::ReplaceCascade,
        insert_batch_size: MIN_INSERT_BATCH_SIZE,
        ..KdjRunRequest::default()
    };

    let summary = run_kdj(&mut executor, &request).unwrap();

    assert_eq!(summary.partition_replace.years, vec![2026, 2027]);
    assert_eq!(executor.multi_queries.len(), 2);
    assert_eq!(executor.multi_queries[0].len(), 2);
    assert_eq!(executor.multi_queries[1].len(), 2);
    assert!(
        executor.multi_queries[1]
            .iter()
            .all(|sql| sql.contains("REPLACE PARTITION"))
    );
}

#[test]
fn request_validation_rejects_small_production_batches() {
    let request = KdjRunRequest {
        request_from: "2026-01-01".to_string(),
        request_to: "2026-01-03".to_string(),
        mode: KdjWriteMode::AppendLatest,
        insert_batch_size: 10,
        ..KdjRunRequest::default()
    };

    let error = request.validate().unwrap_err();

    assert!(matches!(error, FurnaceIoError::InvalidRequest(_)));
}
