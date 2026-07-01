use super::*;
#[test]
fn run_ma_dry_run_reads_close_inputs_and_computes_summary() {
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
    let mut executor = FakeExecutor::with_responses(vec![
        response(security_codes(&["sh.600000"])),
        response(count(0)),
        response(optional_date(Some("2026-01-01"))),
        response(ma_input_rows(&row_refs)),
    ]);
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
    assert!(
        summary
            .to_json()
            .contains("\"price_ma_windows\":[3,5,6,10,12,14,20,24,28,30,57,60,114,250]")
    );
    assert!(executor.queries.iter().any(|query| {
        query.contains("close_price_forward_adj")
            && query.contains("CAST(unadj.volume, 'Nullable(Float64)')")
            && query.contains("ORDER BY adj.security_code, adj.trade_date")
    }));
}

#[test]
fn run_ma_with_previous_state_uses_per_security_valid_price_and_volume_lookback() {
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
    let mut executor = FakeExecutor::with_responses(vec![
        response(count(1)),
        response(ma_previous_states(&[(
            "sh.600000",
            "2026-01-10",
            10.0,
            9.0,
        )])),
        response(optional_date(Some("2025-01-01"))),
        response(ma_input_rows(&row_refs)),
    ]);
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
    let state_query = executor
        .queries
        .iter()
        .find(|query| query.contains("price_ema1_10_state IS NOT NULL"))
        .expect("MA previous-state query should be issued");
    assert!(state_query.contains("assumeNotNull(price_ema1_10_state)"));
    assert!(state_query.contains("assumeNotNull(price_ema2_10_state)"));
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
    let mut executor = FakeExecutor::with_responses(vec![
        response(count(0)),
        response(optional_date(Some("2026-01-01"))),
        response(ma_input_rows(&row_refs)),
        response(count(0)),
    ]);
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
    assert!(executor.queries.iter().any(|query| query.contains(
        "ALTER TABLE fleur_calculation.calc_stock_ma_daily ADD COLUMN IF NOT EXISTS price_ma_30"
    )));
    assert_eq!(executor.inserts.len(), 1);
    assert_eq!(executor.inserts[0].table, DEFAULT_MA_OUTPUT_TABLE);
    assert_eq!(executor.inserts[0].rows, 20);
    assert!(executor.inserts[0].row_type.ends_with("MaInsertRow"));
}

#[test]
fn run_ma_replace_cascade_uses_staging_and_replaces_partitions() {
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
    let mut executor = FakeExecutor::with_responses(vec![
        response(optional_date(Some("2026-01-20"))),
        response(count(0)),
        response(optional_date(Some("2026-01-01"))),
        response(ma_input_rows(&row_refs)),
        response(count(0)),
        response(count(0)),
    ]);
    let request = MaRunRequest {
        request_from: "2026-01-01".to_string(),
        request_to: "2026-01-20".to_string(),
        symbols: vec!["sh.600000".to_string()],
        run_id: Some("replace-ma-test".to_string()),
        mode: MaWriteMode::ReplaceCascade,
        insert_batch_size: MIN_INSERT_BATCH_SIZE,
        ..MaRunRequest::default()
    };

    let summary = run_ma(&mut executor, &request).unwrap();

    assert!(summary.writes_applied);
    assert_eq!(summary.partition_replace.years, vec![2026]);
    assert_eq!(summary.staging_validation, ValidationSummary::passed());
    let staging_table = summary.staging_table.as_deref().unwrap();
    assert!(staging_table.contains("replace_ma_test"));
    assert_eq!(executor.inserts.len(), 1);
    assert_eq!(executor.inserts[0].table, staging_table);
    assert_eq!(executor.inserts[0].rows, 20);
    assert_eq!(executor.multi_queries.len(), 2);
    assert!(
        executor.multi_queries[1]
            .iter()
            .any(|sql| sql.contains("REPLACE PARTITION 2026"))
    );
}

#[test]
fn ma_result_row_converts_to_clickhouse_insert_row() {
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
        price_ma_30: None,
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
    let insert = MaInsertRow::try_from(&row).unwrap();

    assert_eq!(insert.security_code, "sh.600000");
    assert_eq!(
        insert.trade_date,
        parse_clickhouse_date("2026-01-03").unwrap()
    );
    assert_eq!(insert.price_ma_3, Some(1.0));
    assert_eq!(insert.price_ma_5, None);
    assert_eq!(insert.price_ma_57, Some(57.0));
    assert_eq!(insert.price_avg_ma_14_28_57_114, Some(2.0));
    assert_eq!(insert.price_ema2_10_state, Some(4.0));
    assert_eq!(insert.volume_ma_5, Some(5.0));
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
