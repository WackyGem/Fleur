# scheduler

Dagster-based data pipeline scheduler for A-share market data ingestion and processing.

## Overview

This project orchestrates data pipelines for:
- **BaoStock**: A-share stock basic info and daily K-line data
- **EastMoney**: F10 financial statements (balance, income, cashflow, dividends, equity)
- **HTTP Resources**: Trade calendars, market events, limit-up pools
- **Jiuyan Industry OCR**: Industry classification image download and OCR processing

## Architecture

```
pipeline/scheduler/src/scheduler/defs/
├── baostock/              # BaoStock TCP client assets
├── http_resources/        # HTTP API clients and assets
│   ├── eastmoney/         # EastMoney F10 financial data
│   ├── client.py          # AioHttpClient with retry/backoff
│   └── schedules.py       # Dagster schedules and sensors
├── jiuyan_industry_ocr/   # Industry image OCR pipeline
│   ├── assets.py          # Dagster asset definitions
│   ├── services.py        # Service layer (download, OCR)
│   ├── postgres.py        # Repository pattern for DB
│   └── image_store.py     # S3 object store for images
├── io_managers/           # Custom IO managers
└── config.py              # Environment configuration
```

### Key Patterns

- **Repository Pattern**: `PostgresIndustryImageRepository` encapsulates all database operations
- **Object Store**: `ImageObjectStore` provides S3 filesystem abstraction
- **Service Layer**: Business logic extracted into `services.py` for testability
- **Type Safety**: TypedDict for configs, accurate types throughout (minimal `Any` usage)

## Getting Started

### Prerequisites

- Python 3.12+
- PostgreSQL (for OCR state management)
- S3-compatible storage (RustFS/MinIO)
- Docker (for local development)

### Installation

```bash
cd pipeline
uv sync
```

### Configuration

Copy `.env.example` to `.env` and configure:

```bash
cp .env.example .env
# Edit .env with your credentials
```

### Running Dagster

```bash
cd pipeline
uv run dg dev
```

Open http://localhost:3000 to access the Dagster UI.

## Quality Gates

Run these commands before committing:

```bash
# Linting
uv run ruff check scheduler/src scheduler/tests migrate

# Formatting
uv run ruff format scheduler/src scheduler/tests migrate

# Type checking
uv run pyright scheduler/src scheduler/tests

# Tests
uv run pytest scheduler/tests --cov=scheduler/src/scheduler --cov-report=term-missing
```

## Asset Groups

### baostock

BaoStock TCP client for A-share market data.

- `baostock__query_stock_basic`: Stock basic information (latest snapshot)
- `baostock__query_history_k_data_plus_daily`: Daily K-line data (yearly partitions)

### eastmoney

EastMoney F10 financial statements.

- `eastmoney__balance`: Balance sheet
- `eastmoney__income_sq`: Single-quarter income statement
- `eastmoney__income_ytd`: Year-to-date income statement
- `eastmoney__cashflow_sq`: Single-quarter cashflow statement
- `eastmoney__cashflow_ytd`: Year-to-date cashflow statement
- `eastmoney__dividend_allotment`: Dividend allotment events
- `eastmoney__dividend_main`: Main dividend plans
- `eastmoney__equity_history`: Equity change history

### http_resources

HTTP API resources for market events and calendars.

- `sina__trade_calendar`: Sina trade calendar
- `jiuyan__action_field`: Jiuyan action field events
- `jiuyan__industry_list`: Jiuyan industry classification list
- `ths__limit_up_pool`: THS limit-up pool

### jiuyan_industry_ocr

Industry classification image processing pipeline.

- `jiuyan__industry_images`: Download industry classification images
- `jiuyan__industry_ocr`: OCR processing of downloaded images

## Testing

```bash
# Run all tests
uv run pytest scheduler/tests

# Run specific test file
uv run pytest scheduler/tests/test_jiuyan_industry_ocr_services.py

# Run with coverage
uv run pytest scheduler/tests --cov=scheduler/src/scheduler --cov-report=term-missing
```

## Database Migrations

OCR state is managed in PostgreSQL. Run migrations:

```bash
cd pipeline/migrate
uv run alembic upgrade head
```

## Observability

All assets emit standardized metadata:

- `row_count`: Number of rows processed
- `column_count`: Number of columns in output
- `asset_function_seconds`: Total execution time
- `request_count`: Number of API requests made
- `retry_count`: Number of retries performed
- `success_count`/`failure_count`: Success/failure counts
- `s3_keys_sample`: Sample of S3 object keys written

## Failure Strategies

### baostock Assets

BaoStock uses a TCP-based protocol with exponential backoff retry:

- **Retry Policy**: 3 attempts with exponential backoff (1s, 2s, 4s base delays)
- **Connection Failures**: Automatic reconnection on TCP errors
- **Empty Results**: Asset fails if no data returned for requested date range
- **Timeout**: 30 seconds per request

**Recovery**: Dagster retries the entire asset on failure. Manual intervention may be needed for persistent connection issues.

### eastmoney Assets

EastMoney uses HTTP API with retry and concurrency control:

- **Retry Policy**: 5 attempts with exponential backoff for 5xx errors
- **HTTP 4xx**: No retry (client errors)
- **Concurrency**: Limited to `EASTMONEY_CODE_CONCURRENCY` (10) parallel requests per stock code
- **Empty Results**: Allowed with `allow_empty: True` metadata flag
- **Rate Limiting**: Respects 429 responses with backoff

**Recovery**: Automatic retry handles transient failures. Persistent failures require checking API availability or credentials.

### http_resources Assets

HTTP resources use `AioHttpClient` with configurable retry:

- **Retry Policy**: 3 attempts with exponential backoff (default)
- **Circuit Breaker**: Opens after 5 consecutive failures, half-open after 60s
- **Timeout**: 30 seconds per request (configurable per asset)
- **Rate Limiting**: 429 responses trigger automatic backoff

**Recovery**: Circuit breaker prevents cascading failures. Check upstream service health on persistent failures.

### jiuyan_industry_ocr Assets

OCR pipeline uses PostgreSQL state machine with selective retry:

#### jiuyan__industry_images

- **Download Retry**: 3 attempts with exponential backoff per image
- **Partial Success**: Asset succeeds if at least one image downloads successfully
- **Database State**: Tracks download status per image (pending/success/failed)
- **Idempotency**: Skips already-downloaded images unless `force_download=True`

**Recovery**: Failed downloads remain in `pending` state for next run. Use `force_download=True` to retry failed images.

#### jiuyan__industry_ocr

- **OCR Retry**: Configurable via `JIUYAN_OCR_MAX_RETRIES` (default: 3)
- **State Machine**: `pending` → `running` → `success`/`failed`
- **Stale Detection**: Claims images stuck in `running` for > 1 hour (configurable)
- **Force OCR**: Re-processes successful images when `force_ocr=True`
- **Concurrency**: Limited to `max_concurrent_requests` (default: 5)

**Recovery**: 
- Stale images automatically reclaimed on next run
- Failed images remain in `failed` state for manual inspection
- Use `force_ocr=True` to reprocess specific images

**Metadata Tracking**:
- `ocr_success_count`, `ocr_failure_count`: Track per-run results
- `ocr_result_row_count`: Total OCR result rows
- `table_convert_seconds`: Time spent converting OCR to tables
- `result_s3_keys_sample`: Sample of result parquet files

## License

Internal use only.
