use super::*;
#[test]
fn run_boll_dry_run_reads_close_inputs_and_computes_summary() {
    let rows = (1..=20)
        .map(|day| {
            (
                "sh.600000",
                format!("2026-01-{day:02}"),
                if day == 11 { None } else { Some(day as f64) },
            )
        })
        .collect::<Vec<_>>();
    let row_refs = rows
        .iter()
        .map(|(security_code, trade_date, close)| (*security_code, trade_date.as_str(), *close))
        .collect::<Vec<_>>();
    let mut executor = FakeExecutor::with_responses(vec![
        response(security_codes(&["sh.600000"])),
        response(optional_date(Some("2026-01-01"))),
        response(close_input_rows(&row_refs)),
    ]);
    let request = BollRunRequest {
        request_from: "2026-01-01".to_string(),
        request_to: "2026-01-20".to_string(),
        ..BollRunRequest::default()
    };

    let summary = run_boll(&mut executor, &request).unwrap();

    assert_eq!(summary.input_rows, 20);
    assert_eq!(summary.output_rows, 20);
    assert_eq!(summary.input_valid_close_rows, 19);
    assert_eq!(summary.output_valid_close_rows, 19);
    assert!(summary.null_indicator_rows > 0);
    assert_eq!(summary.state_source, "rolling-lookback");
    assert!(summary.to_json().contains("\"indicator\":\"boll\""));
    assert!(summary.to_json().contains("\"stddev_ddof\":0"));
    assert!(summary.to_json().contains("\"field_suffix\":\"10_1p5\""));
    assert!(executor.queries.iter().any(|query| {
        query.contains("close_price_forward_adj")
            && query.contains("ORDER BY security_code, trade_date")
    }));
}

#[test]
fn parallel_boll_outputs_match_serial_outputs() {
    let request = BollRunRequest {
        request_from: "2026-01-01".to_string(),
        request_to: "2026-02-20".to_string(),
        ..BollRunRequest::default()
    };
    let groups = vec![
        BollGroupedInput {
            security_code: "sh.600000".to_string(),
            inputs: (1..=51)
                .map(|day| BollInput::new(format!("2026-02-{day:02}"), Some(day as f64)))
                .collect(),
        },
        BollGroupedInput {
            security_code: "sz.000001".to_string(),
            inputs: (1..=51)
                .map(|day| BollInput::new(format!("2026-02-{day:02}"), Some((day + 20) as f64)))
                .collect(),
        },
    ];

    let mut serial = calculate_boll_grouped_outputs_serial_with_collection(
        &request,
        "2026-02-20",
        &groups,
        true,
    )
    .unwrap()
    .rows;
    let mut parallel = calculate_boll_grouped_outputs_parallel_with_collection(
        &request,
        "2026-02-20",
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
fn run_boll_append_latest_inserts_result_rows() {
    let rows = (1..=20)
        .map(|day| ("sh.600000", format!("2026-01-{day:02}"), Some(day as f64)))
        .collect::<Vec<_>>();
    let row_refs = rows
        .iter()
        .map(|(security_code, trade_date, close)| (*security_code, trade_date.as_str(), *close))
        .collect::<Vec<_>>();
    let mut executor = FakeExecutor::with_responses(vec![
        response(optional_date(Some("2026-01-01"))),
        response(close_input_rows(&row_refs)),
        response(count(0)),
    ]);
    let request = BollRunRequest {
        request_from: "2026-01-01".to_string(),
        request_to: "2026-01-20".to_string(),
        symbols: vec!["sh.600000".to_string()],
        mode: BollWriteMode::AppendLatest,
        insert_batch_size: MIN_INSERT_BATCH_SIZE,
        ..BollRunRequest::default()
    };

    let summary = run_boll(&mut executor, &request).unwrap();

    assert!(summary.writes_applied);
    assert_eq!(executor.inserts.len(), 1);
    assert_eq!(executor.inserts[0].table, DEFAULT_BOLL_OUTPUT_TABLE);
    assert_eq!(executor.inserts[0].rows, 20);
    assert!(executor.inserts[0].row_type.ends_with("BollInsertRow"));
}

#[test]
fn run_boll_replace_cascade_uses_staging_and_replaces_partitions() {
    let rows = (1..=20)
        .map(|day| ("sh.600000", format!("2026-01-{day:02}"), Some(day as f64)))
        .collect::<Vec<_>>();
    let row_refs = rows
        .iter()
        .map(|(security_code, trade_date, close)| (*security_code, trade_date.as_str(), *close))
        .collect::<Vec<_>>();
    let mut executor = FakeExecutor::with_responses(vec![
        response(optional_date(Some("2026-01-20"))),
        response(optional_date(Some("2026-01-01"))),
        response(close_input_rows(&row_refs)),
        response(count(0)),
        response(count(0)),
    ]);
    let request = BollRunRequest {
        request_from: "2026-01-01".to_string(),
        request_to: "2026-01-20".to_string(),
        symbols: vec!["sh.600000".to_string()],
        run_id: Some("replace-boll-test".to_string()),
        mode: BollWriteMode::ReplaceCascade,
        insert_batch_size: MIN_INSERT_BATCH_SIZE,
        ..BollRunRequest::default()
    };

    let summary = run_boll(&mut executor, &request).unwrap();

    assert!(summary.writes_applied);
    assert_eq!(summary.partition_replace.years, vec![2026]);
    assert_eq!(summary.staging_validation, ValidationSummary::passed());
    let staging_table = summary.staging_table.as_deref().unwrap();
    assert!(staging_table.contains("replace_boll_test"));
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
fn boll_result_row_converts_to_clickhouse_insert_row() {
    let row = BollResultRow {
        security_code: "sh.600000".to_string(),
        trade_date: "2026-01-03".to_string(),
        boll_mid_10_1p5: Some(1.0),
        boll_up_10_1p5: Some(2.0),
        boll_dn_10_1p5: Some(0.0),
        boll_mid_20_2: None,
        boll_up_20_2: None,
        boll_dn_20_2: None,
        boll_mid_50_2p5: Some(3.0),
        boll_up_50_2p5: Some(4.0),
        boll_dn_50_2p5: Some(5.0),
    };
    let insert = BollInsertRow::try_from(&row).unwrap();

    assert_eq!(insert.security_code, "sh.600000");
    assert_eq!(
        insert.trade_date,
        parse_clickhouse_date("2026-01-03").unwrap()
    );
    assert_eq!(insert.boll_mid_10_1p5, Some(1.0));
    assert_eq!(insert.boll_up_10_1p5, Some(2.0));
    assert_eq!(insert.boll_dn_10_1p5, Some(0.0));
    assert_eq!(insert.boll_mid_20_2, None);
    assert_eq!(insert.boll_dn_50_2p5, Some(5.0));
}
