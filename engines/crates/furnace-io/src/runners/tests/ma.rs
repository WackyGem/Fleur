use super::*;
#[test]
fn run_ma_dry_run_reads_close_inputs_and_computes_summary() {
    let responses = ["sh.600000\n", "2026-01-01\n", ""];
    let rows = (1..=20)
        .map(|day| {
            (
                "sh.600000",
                format!("2026-01-{day:02}"),
                if day == 11 { None } else { Some(day as f64) },
                if day == 12 {
                    None
                } else {
                    Some((day * 100) as f64)
                },
            )
        })
        .collect::<Vec<_>>();
    let row_refs = rows
        .iter()
        .map(|(security_code, trade_date, close, volume)| {
            (*security_code, trade_date.as_str(), *close, *volume)
        })
        .collect::<Vec<_>>();
    let input_rows = ma_rowbinary_input_rows(&row_refs);
    let mut executor = FakeExecutor::with_responses_and_bytes(&responses, vec![input_rows]);
    let request = MaRunRequest {
        request_from: "2026-01-01".to_string(),
        request_to: "2026-01-20".to_string(),
        ..MaRunRequest::default()
    };

    let summary = run_ma(&mut executor, &request).unwrap();

    assert_eq!(summary.input_rows, 20);
    assert_eq!(summary.output_rows, 20);
    assert_eq!(summary.valid_close_rows, 19);
    assert_eq!(summary.valid_volume_rows, 19);
    assert!(summary.null_indicator_rows > 0);
    assert_eq!(summary.ema_state_source, "full-history");
    assert!(summary.to_json().contains("\"indicator\":\"ma\""));
    assert!(
        summary
            .to_json()
            .contains("\"volume_ma_windows\":[5,10,20,60]")
    );
    assert!(executor.queries.iter().any(|query| {
        query.contains("close_price_forward_adj")
            && query.contains("CAST(unadj.volume, 'Nullable(Float64)')")
            && query.contains("ORDER BY adj.security_code, adj.trade_date")
            && query.contains("FORMAT RowBinary")
    }));
}

#[test]
fn run_ma_with_previous_state_uses_per_security_valid_price_and_volume_lookback() {
    let responses = ["1\n", "sh.600000\t2026-01-10\t10\t9\n", "2025-01-01\n"];
    let rows = (1..=20)
        .map(|day| {
            (
                "sh.600000",
                format!("2026-01-{day:02}"),
                Some(day as f64),
                Some((day * 100) as f64),
            )
        })
        .collect::<Vec<_>>();
    let row_refs = rows
        .iter()
        .map(|(security_code, trade_date, close, volume)| {
            (*security_code, trade_date.as_str(), *close, *volume)
        })
        .collect::<Vec<_>>();
    let input_rows = ma_rowbinary_input_rows(&row_refs);
    let mut executor = FakeExecutor::with_responses_and_bytes(&responses, vec![input_rows]);
    let request = MaRunRequest {
        request_from: "2026-01-11".to_string(),
        request_to: "2026-01-20".to_string(),
        symbols: vec!["sh.600000".to_string()],
        output_table: "fleur_calculation.calc_stock_ma_daily_validation".to_string(),
        ..MaRunRequest::default()
    };

    let summary = run_ma(&mut executor, &request).unwrap();

    assert_eq!(summary.input_from, "2025-01-01");
    assert_eq!(summary.ema_state_source, "previous-state");
    let lookback_query = executor
        .queries
        .iter()
        .find(|query| query.contains("rn <= 250") && query.contains("rn <= 60"))
        .expect("MA lookback query should use explicit valid-row windows");
    assert!(lookback_query.contains("PARTITION BY security_code ORDER BY trade_date DESC"));
    assert!(lookback_query.contains("close_price_forward_adj IS NOT NULL"));
    assert!(
        lookback_query
            .contains("LEFT JOIN fleur_intermediate.int_stock_quotes_daily_unadj AS unadj")
    );
    assert!(lookback_query.contains("unadj.volume IS NOT NULL"));
}
#[test]
fn parallel_ma_outputs_match_serial_outputs() {
    let request = MaRunRequest {
        request_from: "2026-01-01".to_string(),
        request_to: "2026-01-20".to_string(),
        ..MaRunRequest::default()
    };
    let groups = vec![
        MaGroupedInput {
            security_code: "sh.600000".to_string(),
            inputs: (1..=20)
                .map(|day| {
                    MaInput::new(
                        format!("2026-01-{day:02}"),
                        Some(day as f64),
                        Some((day * 100) as f64),
                    )
                })
                .collect(),
        },
        MaGroupedInput {
            security_code: "sz.000001".to_string(),
            inputs: (1..=20)
                .map(|day| {
                    MaInput::new(
                        format!("2026-01-{day:02}"),
                        Some((day + 20) as f64),
                        Some((day * 200) as f64),
                    )
                })
                .collect(),
        },
    ];

    let mut serial = calculate_ma_grouped_outputs_serial_with_collection(
        &request,
        "2026-01-20",
        &groups,
        &HashMap::new(),
        true,
    )
    .unwrap()
    .rows;
    let mut parallel = calculate_ma_grouped_outputs_parallel_with_collection(
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
fn run_ma_append_latest_inserts_result_rows() {
    let responses = ["2026-01-01\n", "", "0\n"];
    let rows = (1..=20)
        .map(|day| {
            (
                "sh.600000",
                format!("2026-01-{day:02}"),
                Some(day as f64),
                Some((day * 100) as f64),
            )
        })
        .collect::<Vec<_>>();
    let row_refs = rows
        .iter()
        .map(|(security_code, trade_date, close, volume)| {
            (*security_code, trade_date.as_str(), *close, *volume)
        })
        .collect::<Vec<_>>();
    let input_rows = ma_rowbinary_input_rows(&row_refs);
    let mut executor = FakeExecutor::with_responses_and_bytes(&responses, vec![input_rows]);
    let request = MaRunRequest {
        request_from: "2026-01-01".to_string(),
        request_to: "2026-01-20".to_string(),
        symbols: vec!["sh.600000".to_string()],
        mode: MaWriteMode::AppendLatest,
        insert_batch_size: MIN_INSERT_BATCH_SIZE,
        ..MaRunRequest::default()
    };

    let summary = run_ma(&mut executor, &request).unwrap();

    assert!(summary.writes_applied);
    assert_eq!(executor.byte_inserts.len(), 1);
    assert!(executor.byte_inserts[0].0.contains("calc_stock_ma_daily"));
    assert!(executor.byte_inserts[0].0.contains("price_ema2_10_state"));
    assert!(executor.byte_inserts[0].0.contains("volume_ma_5"));
    assert!(executor.byte_inserts[0].1.starts_with(b"\tsh.600000"));
}

#[test]
fn ma_result_row_writes_clickhouse_rowbinary_encoding() {
    let row = MaResultRow {
        security_code: "sh.600000".to_string(),
        trade_date: "2026-01-03".to_string(),
        price_ma_3: Some(1.0),
        price_ma_5: None,
        price_ma_6: None,
        price_ma_10: None,
        price_ma_12: None,
        price_ma_14: None,
        price_ma_20: None,
        price_ma_24: None,
        price_ma_28: None,
        price_ma_57: Some(57.0),
        price_ma_60: None,
        price_ma_114: None,
        price_ma_250: None,
        price_avg_ma_3_6_12_24: None,
        price_avg_ma_14_28_57_114: Some(2.0),
        price_ema1_10_state: Some(3.0),
        price_ema2_10: Some(4.0),
        price_ema2_10_state: Some(4.0),
        volume_ma_5: Some(5.0),
        volume_ma_10: None,
        volume_ma_20: None,
        volume_ma_60: None,
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
        None
    );
    for _ in 0..7 {
        assert_eq!(
            read_rowbinary_nullable_f64(&bytes, &mut cursor).unwrap(),
            None
        );
    }
    assert_eq!(
        read_rowbinary_nullable_f64(&bytes, &mut cursor).unwrap(),
        Some(57.0)
    );
}
#[test]
fn ma_request_validation_rejects_non_canonical_price_column_for_writes() {
    let request = MaRunRequest {
        request_from: "2026-01-01".to_string(),
        request_to: "2026-01-03".to_string(),
        mode: MaWriteMode::AppendLatest,
        price_column: "close_price".to_string(),
        insert_batch_size: MIN_INSERT_BATCH_SIZE,
        ..MaRunRequest::default()
    };

    let error = request.validate().unwrap_err();

    assert!(matches!(error, FurnaceIoError::InvalidRequest(_)));
}
