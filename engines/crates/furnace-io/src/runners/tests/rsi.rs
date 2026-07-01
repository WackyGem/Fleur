use super::*;
#[test]
fn run_rsi_dry_run_reads_close_inputs_and_computes_summary() {
    let rows = (1..=51)
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
    let request = RsiRunRequest {
        request_from: "2026-01-01".to_string(),
        request_to: "2026-02-20".to_string(),
        ..RsiRunRequest::default()
    };

    let summary = run_rsi(&mut executor, &request).unwrap();

    assert_eq!(summary.input_rows, 51);
    assert_eq!(summary.output_rows, 51);
    assert_eq!(summary.valid_close_rows, 51);
    assert!(summary.null_indicator_rows > 0);
    assert_eq!(summary.rsi_state_source, "full-history");
    assert!(summary.to_json().contains("\"indicator\":\"rsi\""));
    assert!(executor.queries.iter().any(|query| {
        query.contains("close_price_forward_adj")
            && query.contains("ORDER BY security_code, trade_date")
    }));
}
#[test]
fn parallel_rsi_outputs_match_serial_outputs() {
    let request = RsiRunRequest {
        request_from: "2026-01-01".to_string(),
        request_to: "2026-01-20".to_string(),
        ..RsiRunRequest::default()
    };
    let groups = vec![
        RsiGroupedInput {
            security_code: "sh.600000".to_string(),
            inputs: (1..=20)
                .map(|day| RsiInput::new(format!("2026-01-{day:02}"), Some(day as f64)))
                .collect(),
        },
        RsiGroupedInput {
            security_code: "sz.000001".to_string(),
            inputs: (1..=20)
                .map(|day| RsiInput::new(format!("2026-01-{day:02}"), Some((day + 20) as f64)))
                .collect(),
        },
    ];

    let mut serial = calculate_rsi_grouped_outputs_serial_with_collection(
        &request,
        "2026-01-20",
        &groups,
        &HashMap::new(),
        true,
    )
    .unwrap()
    .rows;
    let mut parallel = calculate_rsi_grouped_outputs_parallel_with_collection(
        &request,
        "2026-01-20",
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
fn run_rsi_append_latest_inserts_result_rows() {
    let rows = (1..=51)
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
    let request = RsiRunRequest {
        request_from: "2026-01-01".to_string(),
        request_to: "2026-02-20".to_string(),
        symbols: vec!["sh.600000".to_string()],
        mode: RsiWriteMode::AppendLatest,
        insert_batch_size: MIN_INSERT_BATCH_SIZE,
        ..RsiRunRequest::default()
    };

    let summary = run_rsi(&mut executor, &request).unwrap();

    assert!(summary.writes_applied);
    assert_eq!(executor.inserts.len(), 1);
    assert_eq!(executor.inserts[0].table, DEFAULT_RSI_OUTPUT_TABLE);
    assert_eq!(executor.inserts[0].rows, 51);
    assert!(executor.inserts[0].row_type.ends_with("RsiInsertRow"));
}

#[test]
fn run_rsi_replace_cascade_uses_staging_and_replaces_partitions() {
    let rows = (1..=51)
        .map(|day| ("sh.600000", fixture_trade_date(day), Some(day as f64)))
        .collect::<Vec<_>>();
    let row_refs = rows
        .iter()
        .map(|(security_code, trade_date, close)| (*security_code, trade_date.as_str(), *close))
        .collect::<Vec<_>>();
    let mut executor = FakeExecutor::with_responses(vec![
        response(optional_date(Some("2026-02-20"))),
        response(optional_date(Some("2026-01-01"))),
        response(count(0)),
        response(close_input_rows(&row_refs)),
        response(count(0)),
        response(count(0)),
    ]);
    let request = RsiRunRequest {
        request_from: "2026-01-01".to_string(),
        request_to: "2026-02-20".to_string(),
        symbols: vec!["sh.600000".to_string()],
        run_id: Some("replace-rsi-test".to_string()),
        mode: RsiWriteMode::ReplaceCascade,
        insert_batch_size: MIN_INSERT_BATCH_SIZE,
        ..RsiRunRequest::default()
    };

    let summary = run_rsi(&mut executor, &request).unwrap();

    assert!(summary.writes_applied);
    assert_eq!(summary.partition_replace.years, vec![2026]);
    assert_eq!(summary.staging_validation, ValidationSummary::passed());
    let staging_table = summary.staging_table.as_deref().unwrap();
    assert!(staging_table.contains("replace_rsi_test"));
    assert_eq!(executor.inserts.len(), 1);
    assert_eq!(executor.inserts[0].table, staging_table);
    assert_eq!(executor.inserts[0].rows, 51);
    assert_eq!(executor.multi_queries.len(), 2);
    assert!(
        executor.multi_queries[1]
            .iter()
            .any(|sql| sql.contains("REPLACE PARTITION 2026"))
    );
}

#[test]
fn run_rsi_append_latest_rejects_previous_state_gaps() {
    let mut executor = FakeExecutor::with_responses(vec![
        response(optional_date(Some("2026-01-01"))),
        response(count(1)),
        response(rsi_previous_states(&[(
            "sh.600000",
            "2026-01-10",
            10.0,
            0.0,
            1.0,
            0.0,
            1.0,
            0.0,
            1.0,
            0.0,
            1.0,
            0.0,
            1.0,
            0.0,
            1.0,
        )])),
        response(gap_count(1, Some("2026-01-11"))),
    ]);
    let request = RsiRunRequest {
        request_from: "2026-01-20".to_string(),
        request_to: "2026-01-21".to_string(),
        symbols: vec!["sh.600000".to_string()],
        mode: RsiWriteMode::AppendLatest,
        insert_batch_size: MIN_INSERT_BATCH_SIZE,
        ..RsiRunRequest::default()
    };

    let error = run_rsi(&mut executor, &request).unwrap_err();

    assert!(error.to_string().contains("RSI result gaps"));
    assert!(error.to_string().contains("2026-01-11"));
    assert!(executor.inserts.is_empty());
    let state_query = executor
        .queries
        .iter()
        .find(|query| query.contains("state_avg_gain_6"))
        .expect("RSI previous-state query should be issued");
    assert!(state_query.contains("assumeNotNull(input.close_price_forward_adj)"));
    assert!(state_query.contains("assumeNotNull(state.state_avg_gain_6)"));
    assert!(state_query.contains("assumeNotNull(state.state_avg_loss_50)"));
    assert!(
        executor
            .queries
            .iter()
            .any(|query| query.contains("countDistinct(input.security_code)"))
    );
}
#[test]
fn rsi_result_row_converts_to_clickhouse_insert_row() {
    let row = RsiResultRow {
        security_code: "sh.600000".to_string(),
        trade_date: "2026-01-03".to_string(),
        rsi_6: Some(1.0),
        rsi_12: None,
        rsi_14: None,
        rsi_24: None,
        rsi_25: None,
        rsi_50: Some(50.0),
        avg_gain_6_state: Some(0.1),
        avg_loss_6_state: Some(0.2),
        avg_gain_12_state: None,
        avg_loss_12_state: None,
        avg_gain_14_state: None,
        avg_loss_14_state: None,
        avg_gain_24_state: None,
        avg_loss_24_state: None,
        avg_gain_25_state: None,
        avg_loss_25_state: None,
        avg_gain_50_state: Some(0.5),
        avg_loss_50_state: Some(0.6),
    };
    let insert = RsiInsertRow::try_from(&row).unwrap();

    assert_eq!(insert.security_code, "sh.600000");
    assert_eq!(
        insert.trade_date,
        parse_clickhouse_date("2026-01-03").unwrap()
    );
    assert_eq!(insert.rsi_6, Some(1.0));
    assert_eq!(insert.rsi_12, None);
    assert_eq!(insert.rsi_50, Some(50.0));
    assert_eq!(insert.avg_gain_6_state, Some(0.1));
    assert_eq!(insert.avg_loss_50_state, Some(0.6));
}
