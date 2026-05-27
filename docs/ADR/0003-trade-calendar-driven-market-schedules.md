# ADR 0003: 市场采集调度以本地交易日历为事实来源

状态：Accepted

日期：2026-05-27

## 背景

A 股行情采集不能只依赖自然日 cron。节假日、调休和非交易日会导致自然日调度产生空 run 或错误请求。Plan 0001 先物化新浪交易日历，Plan 0002 要求 BaoStock 相关调度读取该本地交易日历。

## 决策

市场采集调度以已物化的 `sina__trade_calendar` Parquet 为交易日事实来源。

- `sina__trade_calendar` 每年 12 月 25 到 31 日 09:00 刷新，时区为 `Asia/Shanghai`。
- BaoStock 日常采集 schedule 每天 17:35 评估，时区为 `Asia/Shanghai`。
- schedule 评估时读取 S3 中的 `sina__trade_calendar`，不重新请求新浪远端接口。
- 如果交易日历不存在或不可读，返回 `SkipReason`，提示先 materialize `sina__trade_calendar`。
- 如果评估日期不是 A 股交易日，返回 `SkipReason`，不提交空 run。
- 如果是交易日，BaoStock 日常 job 使用该交易日所在年份作为 K 线分区键，并传入 `refresh_until_trade_date` 限制当年刷新截止日。

## 依据

当前实现：

- `pipeline/scheduler/src/scheduler/defs/http_resources/schedules.py`
- `pipeline/scheduler/src/scheduler/defs/baostock/schedules.py`
- `pipeline/scheduler/src/scheduler/defs/pipeline_defs.py`
- `read_sina_trade_calendar_dates_from_s3`
- `is_trade_date`

设计来源：

- `docs/plans/0001-sina-trade-calendar-s3-ingestion.md`
- `docs/plans/0002-baostock-aio-tcp-client.md`

## 后果

- 交易日判断只有一个事实来源，避免 schedule 与 K 线资产使用不同口径。
- 非交易日不会创建无意义 run。
- 本地交易日历必须先于 BaoStock 日常调度可用。
- 交易日历解析失败时，系统应让 `sina__trade_calendar` materialization 失败，而不是写入空表覆盖有效对象。
- 后续新增市场相关 schedule 时，应复用 `build_trade_date_schedule` 或同等语义。
