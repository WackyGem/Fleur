use super::*;

#[test]
fn run_macd_dry_run_reads_close_inputs_and_computes_summary() {
    let responses = ["sh.600000\n", "2026-01-01\n", "0\n", "0\t\\N\n"];
    let rows = (1..=40)
        .map(|day| ("sh.600000", format!("2026-01-{day:02}"), Some(day as f64)))
        .collect::<Vec<_>>();
    let row_refs = rows
        .iter()
        .map(|(security_code, trade_date, close)| (*security_code, trade_date.as_str(), *close))
        .collect::<Vec<_>>();
    let input_rows = macd_rowbinary_input_rows(&row_refs);
    let mut executor = FakeExecutor::with_responses_and_bytes(&responses, vec![input_rows]);
    let request = MacdRunRequest {
        request_from: "2026-01-01".to_string(),
        request_to: "2026-01-40".to_string(),
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
            && query.contains("FORMAT RowBinary")
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
    let responses = ["2026-01-01\n", "0\n"];
    let rows = (1..=40)
        .map(|day| ("sh.600000", format!("2026-01-{day:02}"), Some(day as f64)))
        .collect::<Vec<_>>();
    let row_refs = rows
        .iter()
        .map(|(security_code, trade_date, close)| (*security_code, trade_date.as_str(), *close))
        .collect::<Vec<_>>();
    let input_rows = macd_rowbinary_input_rows(&row_refs);
    let mut executor = FakeExecutor::with_responses_and_bytes(&responses, vec![input_rows]);
    let request = MacdRunRequest {
        request_from: "2026-01-01".to_string(),
        request_to: "2026-01-40".to_string(),
        symbols: vec!["sh.600000".to_string()],
        mode: MacdWriteMode::AppendLatest,
        insert_batch_size: MIN_INSERT_BATCH_SIZE,
        ..MacdRunRequest::default()
    };

    let summary = run_macd(&mut executor, &request).unwrap();

    assert!(summary.writes_applied);
    assert_eq!(executor.byte_inserts.len(), 1);
    assert!(executor.byte_inserts[0].0.contains("calc_stock_macd_daily"));
    assert!(executor.byte_inserts[0].0.contains("macd_histogram"));
    assert!(executor.byte_inserts[0].1.starts_with(b"\tsh.600000"));
}

#[test]
fn run_macd_append_latest_rejects_previous_state_gaps() {
    let responses = [
        "2026-01-01\n",
        "1\n",
        "sh.600000\t2026-01-10\t1\t2\t0.5\n",
        "0\n",
        "1\t2026-01-11\n",
    ];
    let mut executor = FakeExecutor::with_responses_and_bytes(&responses, Vec::new());
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
    assert!(executor.byte_inserts.is_empty());
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
fn macd_result_row_writes_clickhouse_rowbinary_encoding() {
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
    let mut bytes = Vec::new();

    row.write_row_binary(&mut bytes).unwrap();

    let mut cursor = 0;
    assert_eq!(
        read_rowbinary_string(&bytes, &mut cursor).unwrap(),
        "sh.600000"
    );
    cursor += 2;
    assert_eq!(
        read_rowbinary_nullable_f64(&bytes, &mut cursor).unwrap(),
        Some(1.0)
    );
    assert_eq!(
        read_rowbinary_nullable_f64(&bytes, &mut cursor).unwrap(),
        Some(2.0)
    );
    assert_eq!(
        read_rowbinary_nullable_f64(&bytes, &mut cursor).unwrap(),
        Some(-1.0)
    );
    assert_eq!(
        read_rowbinary_nullable_f64(&bytes, &mut cursor).unwrap(),
        None
    );
}
