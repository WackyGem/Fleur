use super::*;

#[test]
fn run_macd_dry_run_reads_close_inputs_and_computes_summary() {
    let rows = (1..=40)
        .map(|day| ("sh.600000", fixture_trade_date(day), Some(day as f64)))
        .collect::<Vec<_>>();
    let row_refs = rows
        .iter()
        .map(|(security_code, trade_date, close)| (*security_code, trade_date.as_str(), *close))
        .collect::<Vec<_>>();
    let mut executor = FakeExecutor::with_responses(vec![
        response(security_codes(&["sh.600000"])),
        response(optional_date(Some("2026-01-01"))),
        response(count(0)),
        response(close_input_rows(&row_refs)),
    ]);
    let request = MacdRunRequest {
        request_from: "2026-01-01".to_string(),
        request_to: "2026-02-09".to_string(),
        ..MacdRunRequest::default()
    };

    let summary = run_macd(&mut executor, &request).unwrap();

    assert_eq!(summary.input_rows, 40);
    assert_eq!(summary.output_rows, 40);
    assert_eq!(summary.valid_close_rows, 40);
    assert!(summary.null_indicator_rows > 0);
    assert_eq!(summary.macd_state_source, "full-history");
    assert_eq!(summary.incomplete_state_symbols_count, 0);
    assert!(summary.to_json().contains("\"indicator\":\"macd\""));
    assert!(
        summary
            .to_json()
            .contains("\"histogram_mode\":\"DIF - DEA\"")
    );
    assert!(summary.to_json().contains("\"fast_window\":12"));
    assert!(executor.queries.iter().any(|query| {
        query.contains("close_price_forward_adj")
            && query.contains("ORDER BY security_code, trade_date")
    }));
}

#[test]
fn parallel_macd_outputs_match_serial_outputs() {
    let request = MacdRunRequest {
        request_from: "2026-01-01".to_string(),
        request_to: "2026-01-40".to_string(),
        ..MacdRunRequest::default()
    };
    let groups = vec![
        MacdGroupedInput {
            security_code: "sh.600000".to_string(),
            inputs: (1..=40)
                .map(|day| MacdInput::new(format!("2026-01-{day:02}"), Some(day as f64)))
                .collect(),
        },
        MacdGroupedInput {
            security_code: "sz.000001".to_string(),
            inputs: (1..=40)
                .map(|day| MacdInput::new(format!("2026-01-{day:02}"), Some((day + 20) as f64)))
                .collect(),
        },
    ];

    let mut serial = calculate_macd_grouped_outputs_serial_with_collection(
        &request,
        "2026-01-40",
        &groups,
        &HashMap::new(),
        true,
    )
    .unwrap()
    .rows;
    let mut parallel = calculate_macd_grouped_outputs_parallel_with_collection(
        &request,
        "2026-01-40",
        &groups,
        &HashMap::new(),
        true,
    )
    .unwrap()
    .rows;
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
fn run_macd_append_latest_inserts_result_rows() {
    let rows = (1..=40)
        .map(|day| ("sh.600000", fixture_trade_date(day), Some(day as f64)))
        .collect::<Vec<_>>();
    let row_refs = rows
        .iter()
        .map(|(security_code, trade_date, close)| (*security_code, trade_date.as_str(), *close))
        .collect::<Vec<_>>();
    let mut executor = FakeExecutor::with_responses(vec![
        response(optional_date(Some("2026-01-01"))),
        response(count(0)),
        response(close_input_rows(&row_refs)),
        response(count(0)),
    ]);
    let request = MacdRunRequest {
        request_from: "2026-01-01".to_string(),
        request_to: "2026-02-09".to_string(),
        symbols: vec!["sh.600000".to_string()],
        mode: MacdWriteMode::AppendLatest,
        insert_batch_size: MIN_INSERT_BATCH_SIZE,
        ..MacdRunRequest::default()
    };

    let summary = run_macd(&mut executor, &request).unwrap();

    assert!(summary.writes_applied);
    assert_eq!(executor.inserts.len(), 1);
    assert_eq!(executor.inserts[0].table, DEFAULT_MACD_OUTPUT_TABLE);
    assert_eq!(executor.inserts[0].rows, 40);
    assert!(executor.inserts[0].row_type.ends_with("MacdInsertRow"));
}

#[test]
fn run_macd_replace_cascade_uses_staging_and_replaces_partitions() {
    let rows = (1..=40)
        .map(|day| ("sh.600000", fixture_trade_date(day), Some(day as f64)))
        .collect::<Vec<_>>();
    let row_refs = rows
        .iter()
        .map(|(security_code, trade_date, close)| (*security_code, trade_date.as_str(), *close))
        .collect::<Vec<_>>();
    let mut executor = FakeExecutor::with_responses(vec![
        response(optional_date(Some("2026-02-09"))),
        response(optional_date(Some("2026-01-01"))),
        response(count(0)),
        response(close_input_rows(&row_refs)),
        response(count(0)),
        response(count(0)),
    ]);
    let request = MacdRunRequest {
        request_from: "2026-01-01".to_string(),
        request_to: "2026-02-09".to_string(),
        symbols: vec!["sh.600000".to_string()],
        run_id: Some("replace-macd-test".to_string()),
        mode: MacdWriteMode::ReplaceCascade,
        insert_batch_size: MIN_INSERT_BATCH_SIZE,
        ..MacdRunRequest::default()
    };

    let summary = run_macd(&mut executor, &request).unwrap();

    assert!(summary.writes_applied);
    assert_eq!(summary.partition_replace.years, vec![2026]);
    assert_eq!(summary.staging_validation, ValidationSummary::passed());
    let staging_table = summary.staging_table.as_deref().unwrap();
    assert!(staging_table.contains("replace_macd_test"));
    assert_eq!(executor.inserts.len(), 1);
    assert_eq!(executor.inserts[0].table, staging_table);
    assert_eq!(executor.inserts[0].rows, 40);
    assert_eq!(executor.multi_queries.len(), 2);
    assert!(
        executor.multi_queries[1]
            .iter()
            .any(|sql| sql.contains("REPLACE PARTITION 2026"))
    );
}

#[test]
fn run_macd_append_latest_rejects_previous_state_gaps() {
    let mut executor = FakeExecutor::with_responses(vec![
        response(optional_date(Some("2026-01-01"))),
        response(count(1)),
        response(macd_previous_states(&[(
            "sh.600000",
            "2026-01-10",
            1.0,
            2.0,
            0.5,
        )])),
        response(count(0)),
        response(gap_count(1, Some("2026-01-11"))),
    ]);
    let request = MacdRunRequest {
        request_from: "2026-01-20".to_string(),
        request_to: "2026-01-21".to_string(),
        symbols: vec!["sh.600000".to_string()],
        mode: MacdWriteMode::AppendLatest,
        insert_batch_size: MIN_INSERT_BATCH_SIZE,
        ..MacdRunRequest::default()
    };

    let error = run_macd(&mut executor, &request).unwrap_err();

    assert!(error.to_string().contains("MACD result gaps"));
    assert!(error.to_string().contains("2026-01-11"));
    assert!(executor.inserts.is_empty());
    let state_query = executor
        .queries
        .iter()
        .find(|query| query.contains("ema_fast_state_12 IS NOT NULL"))
        .expect("MACD previous-state query should be issued");
    assert!(state_query.contains("assumeNotNull(ema_fast_state_12)"));
    assert!(state_query.contains("assumeNotNull(ema_slow_state_26)"));
    assert!(state_query.contains("assumeNotNull(macd_dea_state)"));
}

#[test]
fn macd_request_rejects_non_default_production_output_table() {
    let request = MacdRunRequest {
        request_from: "2026-01-01".to_string(),
        request_to: "2026-01-02".to_string(),
        mode: MacdWriteMode::AppendLatest,
        output_table: "scratch.calc_stock_macd_daily".to_string(),
        insert_batch_size: MIN_INSERT_BATCH_SIZE,
        ..MacdRunRequest::default()
    };

    let error = request.validate().unwrap_err();

    assert!(
        error
            .to_string()
            .contains("production MACD writes only allow output table")
    );
}

#[test]
fn macd_result_row_converts_to_clickhouse_insert_row() {
    let row = MacdResultRow {
        security_code: "sh.600000".to_string(),
        trade_date: "2026-01-03".to_string(),
        ema_fast_state_12: Some(1.0),
        ema_slow_state_26: Some(2.0),
        macd_dif: Some(-1.0),
        macd_dea: None,
        macd_dea_state: None,
        macd_histogram: None,
    };
    let insert = MacdInsertRow::try_from(&row).unwrap();

    assert_eq!(insert.security_code, "sh.600000");
    assert_eq!(
        insert.trade_date,
        parse_clickhouse_date("2026-01-03").unwrap()
    );
    assert_eq!(insert.ema_fast_state_12, Some(1.0));
    assert_eq!(insert.ema_slow_state_26, Some(2.0));
    assert_eq!(insert.macd_dif, Some(-1.0));
    assert_eq!(insert.macd_dea, None);
    assert_eq!(insert.macd_histogram, None);
}
