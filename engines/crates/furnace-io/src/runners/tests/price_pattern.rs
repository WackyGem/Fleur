use super::*;

#[test]
fn run_price_pattern_dry_run_reads_join_inputs_and_computes_summary() {
    let responses = ["sh.600000\n", "2026-01-01\n"];
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
    let input_rows = price_pattern_rowbinary_input_rows(&rows);
    let mut executor = FakeExecutor::with_responses_and_bytes(&responses, vec![input_rows]);
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
            && query.contains("FORMAT RowBinary")
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
    let responses = ["2026-01-01\n", "0\n"];
    let input_rows = price_pattern_rowbinary_input_rows(&[
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
    ]);
    let mut executor = FakeExecutor::with_responses_and_bytes(&responses, vec![input_rows]);
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
    assert_eq!(executor.byte_inserts.len(), 1);
    assert!(
        executor.byte_inserts[0]
            .0
            .contains(DEFAULT_PRICE_PATTERN_OUTPUT_TABLE)
    );
    assert!(
        executor.byte_inserts[0]
            .0
            .contains("n_structure_20_is_valid")
    );
    assert!(executor.byte_inserts[0].1.starts_with(b"\tsh.600000"));
}

#[test]
fn price_pattern_result_row_writes_clickhouse_rowbinary_encoding() {
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
    let mut bytes = Vec::new();

    row.write_row_binary(&mut bytes).unwrap();

    let mut cursor = 0;
    assert_eq!(
        read_rowbinary_string(&bytes, &mut cursor).unwrap(),
        "sh.600000"
    );
    cursor += 2;
    assert_eq!(bytes[cursor], 0);
    cursor += 1;
    assert_eq!(i8::from_le_bytes([bytes[cursor]]), 1);
    cursor += 1;
    assert_eq!(bytes[cursor], 0);
    cursor += 1;
    assert_eq!(
        u16::from_le_bytes(bytes[cursor..cursor + 2].try_into().unwrap()),
        2
    );
    cursor += 2;
    assert_eq!(bytes[cursor], 0);
    cursor += 1;
    assert_eq!(
        u16::from_le_bytes(bytes[cursor..cursor + 2].try_into().unwrap()),
        0
    );
    cursor += 2;
    assert_eq!(
        u16::from_le_bytes(bytes[cursor..cursor + 2].try_into().unwrap()),
        3
    );
    cursor += 2;
    assert_eq!(bytes[cursor], 0);
    cursor += 3;
    assert_eq!(
        read_rowbinary_nullable_f64(&bytes, &mut cursor).unwrap(),
        Some(15.0)
    );
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
