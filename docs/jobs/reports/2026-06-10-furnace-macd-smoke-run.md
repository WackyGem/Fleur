# Furnace MACD Smoke Run

日期：2026-06-10

范围：

- 输入表：`fleur_intermediate.int_stock_quotes_daily_adj`
- 输出表：`fleur_calculation.calc_stock_macd_daily`
- 证券：`600674.SH`
- 请求区间：`2026-05-01` 至 `2026-06-01`
- 参数：`MACD(12,26,9)`，前复权收盘价 `close_price_forward_adj`
- histogram 口径：`DIF - DEA`

## 环境

本机未安装 `clickhouse-client`，本次通过 Docker 容器内客户端执行：

```bash
FURNACE_CLICKHOUSE_CLIENT=docker
FURNACE_CLICKHOUSE_CLIENT_ARGS='exec -i mono-fleur-clickhouse clickhouse-client'
CLICKHOUSE_HOST=127.0.0.1
CLICKHOUSE_NATIVE_PORT=9000
CLICKHOUSE_USER=mono_fleur
CLICKHOUSE_PASSWORD=change-me-clickhouse-password
```

输入表基线：

```text
rows = 17990764
min_trade_date = 1995-01-03
max_trade_date = 2026-06-01
securities = 5532
```

## Replace Cascade Smoke

命令：

```bash
./engines/target/release/furnace macd \
  --from 2026-05-01 \
  --to 2026-06-01 \
  --symbols 600674.SH \
  --mode replace-cascade \
  --run-id macd_smoke_600674_20260610_1845 \
  --output-format json
```

结果摘要：

```text
request_from = 2026-05-01
request_to = 2026-06-01
effective_output_from = 2026-05-01
effective_output_to = 2026-06-01
input_from = 1995-01-03
input_to = 2026-06-01
input_rows = 7623
output_rows = 19
valid_close_rows = 7623
null_indicator_rows = 0
affected_years = [2026]
retained_rows = 498535
staging_validation.status = passed
staging_validation.duplicate_keys = 0
partition_replace.status = replaced
macd_state_source = full-history
incomplete_state_symbols_count = 1
gap_symbols_count = 0
writes_applied = true
```

性能摘要：

```text
total_ms = 3095
read_input_ms = 179
read_state_ms = 737
group_ms = 2
compute_ms = 3
write_ms = 157
staging_ms = 1076
partition_replace_ms = 164
parallelism = serial
worker_threads = 20
```

写入后全表校验：

```text
rows = 17990764
min_trade_date = 1995-01-03
max_trade_date = 2026-06-01
securities = 5532
duplicate key rows = 0
```

样本证券区间校验：

```text
security_code = 600674.SH
rows = 19
min_trade_date = 2026-05-06
max_trade_date = 2026-06-01
null macd_dif rows = 0
null macd_dea rows = 0
null macd_histogram rows = 0
```

## SMA 启动语义抽样

对 `600674.SH` 按有效 `close_price_forward_adj` 计数：

```text
first_dif = 1995-02-14, valid_close_index = 26
first_dea = 1995-02-24, valid_close_index = 34
first_histogram = 1995-02-24, valid_close_index = 34
```

全表 histogram 口径校验：

```text
mismatched_histogram_rows where abs(macd_histogram - (macd_dif - macd_dea)) > 1e-12 = 0
```

## dbt Wrapper 验收

命令：

```bash
cd pipeline
uv run dbt build --project-dir elt --profiles-dir elt --select int_stock_macd_daily
```

结果：通过，`PASS=8 WARN=0 ERROR=0 SKIP=0 NO-OP=0 TOTAL=8`。

## 结论

- `furnace macd` 能通过 staging + 年度 `REPLACE PARTITION` 写入 `fleur_calculation.calc_stock_macd_daily`。
- 单证券 smoke 写入保留了 2026 分区内不受影响证券的既有全量行。
- dbt `int_stock_macd_daily` wrapper 只暴露 `macd_dif`、`macd_dea`、`macd_histogram`，不重写 MACD 公式。
