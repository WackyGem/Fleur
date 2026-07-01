use super::*;

#[test]
fn run_price_pattern_dry_run_reads_join_inputs_and_computes_summary() {
    let rows = vec![
        (
            "sh.600000",
            "2026-01-01",
            Some(10.0),
            Some(5.0),
            Some(11.0),
            Some(10.0),
        ),
        (
            "sh.600000",
            "2026-01-02",
            Some(15.0),
            Some(7.0),
            Some(12.0),
            Some(11.0),
        ),
        (
            "sh.600000",
            "2026-01-03",
            Some(12.0),
            Some(8.0),
            None,
            Some(12.0),
        ),
    ];
    let mut executor = FakeExecutor::with_responses(vec![
        response(security_codes(&["sh.600000"])),
        response(optional_date(Some("2026-01-01"))),
        response(price_pattern_input_rows(&rows)),
    ]);
    let request = PricePatternRunRequest {
        request_from: "2026-01-01".to_string(),
        request_to: "2026-01-03".to_string(),
        ..PricePatternRunRequest::default()
    };

    let summary = run_price_pattern(&mut executor, &request).unwrap();

    assert_eq!(summary.input_rows, 3);
    assert_eq!(summary.output_rows, 3);
    assert_eq!(summary.input_valid_streak_rows, 2);
    assert_eq!(summary.input_valid_structure_bar_rows, 3);
    assert_eq!(summary.valid_streak_rows, 2);
    assert_eq!(summary.valid_structure_bar_rows, 3);
    assert_eq!(summary.null_streak_rows, 1);
    assert_eq!(summary.state_source, "full-history");
    assert_eq!(summary.n_structure_window, 20);
    assert!(
        summary
            .to_json()
            .contains("\"indicator\":\"price_pattern\"")
    );
    assert!(summary.to_json().contains("\"valid_streak_rows\":2"));
    assert!(executor.queries.iter().any(|query| {
        query.contains("LEFT JOIN fleur_intermediate.int_stock_quotes_daily_unadj AS unadj")
            && query.contains("adj.high_price_forward_adj")
            && query.contains("unadj.prev_close_price")
            && query.contains("ORDER BY adj.security_code, adj.trade_date")
    }));
}

#[test]
fn parallel_price_pattern_outputs_match_serial_outputs() {
    let request = PricePatternRunRequest {
        request_from: "2026-01-01".to_string(),
        request_to: "2026-01-03".to_string(),
        ..PricePatternRunRequest::default()
    };
    let groups = vec![
        PricePatternGroupedInput {
            security_code: "sh.600000".to_string(),
            inputs: vec![
                PricePatternInput::new("2026-01-01", Some(10.0), Some(5.0), Some(11.0), Some(10.0)),
                PricePatternInput::new("2026-01-02", Some(15.0), Some(7.0), Some(12.0), Some(11.0)),
                PricePatternInput::new("2026-01-03", Some(12.0), Some(8.0), Some(13.0), Some(12.0)),
            ],
        },
        PricePatternGroupedInput {
            security_code: "sz.000001".to_string(),
            inputs: vec![
                PricePatternInput::new("2026-01-01", Some(9.0), Some(4.0), Some(10.0), Some(11.0)),
                PricePatternInput::new("2026-01-02", Some(11.0), Some(5.0), Some(9.0), Some(10.0)),
                PricePatternInput::new("2026-01-03", Some(10.0), Some(6.0), Some(8.0), Some(9.0)),
            ],
        },
    ];

    let mut serial = calculate_price_pattern_grouped_outputs_serial_with_collection(
        &request,
        "2026-01-03",
        &groups,
        true,
    )
    .unwrap()
    .rows;
    let mut parallel = calculate_price_pattern_grouped_outputs_parallel_with_collection(
        &request,
        "2026-01-03",
        &groups,
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
fn run_price_pattern_append_latest_inserts_result_rows() {
    let input_rows = [
        (
            "sh.600000",
            "2026-01-01",
            Some(10.0),
            Some(5.0),
            Some(11.0),
            Some(10.0),
        ),
        (
            "sh.600000",
            "2026-01-02",
            Some(15.0),
            Some(7.0),
            Some(12.0),
            Some(11.0),
        ),
    ];
    let mut executor = FakeExecutor::with_responses(vec![
        response(optional_date(Some("2026-01-01"))),
        response(price_pattern_input_rows(&input_rows)),
        response(count(0)),
    ]);
    let request = PricePatternRunRequest {
        request_from: "2026-01-01".to_string(),
        request_to: "2026-01-02".to_string(),
        symbols: vec!["sh.600000".to_string()],
        mode: PricePatternWriteMode::AppendLatest,
        insert_batch_size: MIN_INSERT_BATCH_SIZE,
        ..PricePatternRunRequest::default()
    };

    let summary = run_price_pattern(&mut executor, &request).unwrap();

    assert!(summary.writes_applied);
    assert_eq!(executor.inserts.len(), 1);
    assert_eq!(
        executor.inserts[0].table,
        DEFAULT_PRICE_PATTERN_OUTPUT_TABLE
    );
    assert_eq!(executor.inserts[0].rows, 2);
    assert!(
        executor.inserts[0]
            .row_type
            .ends_with("PricePatternInsertRow")
    );
}

#[test]
fn run_price_pattern_replace_cascade_uses_staging_and_replaces_partitions() {
    let input_rows = [
        (
            "sh.600000",
            "2026-01-01",
            Some(10.0),
            Some(5.0),
            Some(11.0),
            Some(10.0),
        ),
        (
            "sh.600000",
            "2026-01-02",
            Some(15.0),
            Some(7.0),
            Some(12.0),
            Some(11.0),
        ),
    ];
    let mut executor = FakeExecutor::with_responses(vec![
        response(optional_date(Some("2026-01-02"))),
        response(optional_date(Some("2026-01-01"))),
        response(price_pattern_input_rows(&input_rows)),
        response(count(0)),
        response(count(0)),
    ]);
    let request = PricePatternRunRequest {
        request_from: "2026-01-01".to_string(),
        request_to: "2026-01-02".to_string(),
        symbols: vec!["sh.600000".to_string()],
        run_id: Some("replace-price-pattern-test".to_string()),
        mode: PricePatternWriteMode::ReplaceCascade,
        insert_batch_size: MIN_INSERT_BATCH_SIZE,
        ..PricePatternRunRequest::default()
    };

    let summary = run_price_pattern(&mut executor, &request).unwrap();

    assert!(summary.writes_applied);
    assert_eq!(summary.partition_replace.years, vec![2026]);
    assert_eq!(summary.staging_validation, ValidationSummary::passed());
    let staging_table = summary.staging_table.as_deref().unwrap();
    assert!(staging_table.contains("replace_price_pattern_test"));
    assert_eq!(executor.inserts.len(), 1);
    assert_eq!(executor.inserts[0].table, staging_table);
    assert_eq!(executor.inserts[0].rows, 2);
    assert_eq!(executor.multi_queries.len(), 2);
    assert!(
        executor.multi_queries[1]
            .iter()
            .any(|sql| sql.contains("REPLACE PARTITION 2026"))
    );
}

#[test]
fn price_pattern_result_row_converts_to_clickhouse_insert_row() {
    let row = PricePatternResultRow {
        security_code: "sh.600000".to_string(),
        trade_date: "2026-01-03".to_string(),
        close_direction: Some(1),
        close_up_streak_days: Some(2),
        close_down_streak_days: Some(0),
        n_structure_20_valid_bars: 3,
        n_structure_20_high_date: Some("2026-01-02".to_string()),
        n_structure_20_high_price: Some(15.0),
        n_structure_20_low_date: Some("2026-01-01".to_string()),
        n_structure_20_low_price: Some(5.0),
        n_structure_20_second_low_date: Some("2026-01-03".to_string()),
        n_structure_20_second_low_price: Some(8.0),
        n_structure_20_second_low_ratio: Some(1.6),
        n_structure_20_is_valid: true,
    };
    let insert = PricePatternInsertRow::try_from(&row).unwrap();

    assert_eq!(insert.security_code, "sh.600000");
    assert_eq!(
        insert.trade_date,
        parse_clickhouse_date("2026-01-03").unwrap()
    );
    assert_eq!(insert.close_direction, Some(1));
    assert_eq!(insert.close_up_streak_days, Some(2));
    assert_eq!(insert.n_structure_20_valid_bars, 3);
    assert_eq!(
        insert.n_structure_20_high_date,
        Some(parse_clickhouse_date("2026-01-02").unwrap())
    );
    assert_eq!(insert.n_structure_20_high_price, Some(15.0));
    assert!(insert.n_structure_20_is_valid);
}

#[test]
fn create_price_pattern_output_table_contains_canonical_fields() {
    let ddl = create_price_pattern_output_table_sql(DEFAULT_PRICE_PATTERN_OUTPUT_TABLE);

    assert!(ddl.contains("calc_stock_price_pattern_daily"));
    assert!(ddl.contains("close_direction Nullable(Int8)"));
    assert!(ddl.contains("n_structure_20_second_low_ratio Nullable(Float64)"));
    assert!(ddl.contains("n_structure_20_is_valid Bool"));
    assert!(ddl.contains("PARTITION BY toYear(trade_date)"));
    assert!(ddl.contains("ORDER BY (trade_date, security_code)"));
}

#[test]
fn price_pattern_production_rejects_non_canonical_columns() {
    let request = PricePatternRunRequest {
        request_from: "2026-01-01".to_string(),
        request_to: "2026-01-02".to_string(),
        mode: PricePatternWriteMode::AppendLatest,
        close_column: "close_price_forward_adj".to_string(),
        insert_batch_size: MIN_INSERT_BATCH_SIZE,
        ..PricePatternRunRequest::default()
    };

    let error = request.validate().unwrap_err();

    assert!(error.to_string().contains("close column close_price"));
}
