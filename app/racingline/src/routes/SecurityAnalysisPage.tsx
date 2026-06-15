import {
  useCallback,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react"
import { Link, useParams, useSearchParams } from "react-router-dom"
import {
  ArrowLeft01Icon,
  ArrowReloadHorizontalIcon,
  ArrowRight01Icon,
  EyeIcon,
} from "@hugeicons/core-free-icons"

import {
  useBuySignalsQuery,
  usePoolMembersQuery,
  useRunDaysQuery,
  useRunQuery,
  useSecurityAnalysisQuery,
} from "@/api/hooks"
import {
  ErrorState,
  MissingBackendState,
  TableSkeleton,
} from "@/components/racingline/data-state"
import { RacinglineIcon } from "@/components/racingline/icon"
import { SecurityAnalysisChart } from "@/features/analysis/components/security-analysis-chart"
import {
  DEFAULT_ANALYSIS_SOURCE,
  DEFAULT_PRICE_OVERLAYS,
  PRICE_OVERLAY_KEYS,
  buildRunDetailPath,
  buildSecurityAnalysisPath,
  buildSecurityAnalysisQuery,
  parseAnalysisSource,
  parsePriceAdjustment,
  quoteForTradeDate,
  type PriceOverlayKey,
} from "@/features/analysis/security-analysis"
import {
  displayJsonValue,
  formatCount,
  formatScore,
  jsonEntries,
  jsonPreview,
  selectDefaultTradeDate,
  shortId,
} from "@/lib/format"
import { cn } from "@/lib/utils"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Separator } from "@/components/ui/separator"
import { Skeleton } from "@/components/ui/skeleton"
import {
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
} from "@/components/ui/tabs"
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group"
import type {
  AnalysisSource,
  BuySignalRecord,
  JsonRecord,
  JsonValue,
  PoolMemberRecord,
  PriceAdjustment,
  QuoteMartRow,
  ResultSnapshot,
  SecurityAnalysisResponse,
} from "@/types/rearview"

const PAGE_SIZE = 50

const SOURCE_LABELS: Record<AnalysisSource, string> = {
  pool: "Pool",
  signals: "Buy signals",
}

const ADJUSTMENT_OPTIONS = [
  { label: "Forward", value: "forward_adjusted" },
  { label: "Backward", value: "backward_adjusted" },
  { label: "Unadjusted", value: "unadjusted" },
] as const

const PRICE_OVERLAY_LABELS: Record<PriceOverlayKey, string> = {
  price_ma_5: "MA5",
  price_ma_10: "MA10",
  price_ma_30: "MA30",
  price_ema2_10: "EMA2-10",
  price_avg_ma_3_6_12_24: "AVG 3/6/12/24",
  price_avg_ma_14_28_57_114: "AVG 14/28/57/114",
}

const METRIC_PREVIEW_PRIORITY = [
  "kdj_j_value",
  "close_price_forward_adj",
  "pct_change",
  "pct_amplitude",
  "volume",
  "volume_ma_5",
]

type ResultRow = BuySignalRecord | PoolMemberRecord

type QuoteField = {
  key: keyof QuoteMartRow
  label: string
  kind?: "boolean" | "date" | "text"
}

const QUOTE_GROUPS: Array<{ title: string; fields: QuoteField[] }> = [
  {
    title: "OHLC",
    fields: [
      { key: "open_price", label: "open" },
      { key: "high_price", label: "high" },
      { key: "low_price", label: "low" },
      { key: "close_price", label: "close" },
      { key: "prev_close_price", label: "prev close" },
      { key: "pct_change", label: "pct change" },
      { key: "pct_amplitude", label: "amplitude" },
    ],
  },
  {
    title: "Adjusted OHLC",
    fields: [
      { key: "close_price_forward_adj", label: "forward close" },
      { key: "prev_close_price_forward_adj", label: "forward prev" },
      { key: "forward_adjustment_factor", label: "forward factor" },
      { key: "close_price_backward_adj", label: "backward close" },
      { key: "prev_close_price_backward_adj", label: "backward prev" },
      { key: "backward_adjustment_factor", label: "backward factor" },
    ],
  },
  {
    title: "Trading",
    fields: [
      { key: "volume", label: "volume" },
      { key: "prev_volume", label: "prev volume" },
      { key: "amount", label: "amount" },
      { key: "turnover_rate", label: "turnover" },
      { key: "turnover_rate_actual", label: "actual turnover" },
      { key: "limit_up_price", label: "limit up" },
      { key: "limit_down_price", label: "limit down" },
    ],
  },
  {
    title: "Capitalization",
    fields: [
      { key: "a_market_cap", label: "A market cap" },
      { key: "a_float_market_cap", label: "A float cap" },
      { key: "a_free_float_market_cap", label: "A free float cap" },
      { key: "a_shares", label: "A shares" },
      { key: "a_float_shares", label: "A float shares" },
      { key: "a_free_float_shares", label: "A free float shares" },
    ],
  },
  {
    title: "Valuation",
    fields: [
      { key: "pe_static", label: "PE static" },
      { key: "pe_ttm", label: "PE TTM" },
      { key: "pe_forecast", label: "PE forecast" },
      { key: "pb_mrq", label: "PB MRQ" },
      { key: "book_value_per_share", label: "book value/share" },
      { key: "roe", label: "ROE" },
      { key: "roa", label: "ROA" },
      { key: "roaa", label: "ROAA" },
      { key: "roae", label: "ROAE" },
      { key: "dy_static", label: "DY static" },
      { key: "dy_ttm", label: "DY TTM" },
    ],
  },
  {
    title: "Status and KDJ",
    fields: [
      { key: "is_suspend", label: "suspend", kind: "boolean" },
      { key: "is_st", label: "ST", kind: "boolean" },
      { key: "kdj_rsv", label: "RSV" },
      { key: "kdj_k_value", label: "K" },
      { key: "kdj_d_value", label: "D" },
      { key: "kdj_j_value", label: "J" },
    ],
  },
]

export function SecurityAnalysisPage() {
  const { runId, securityCode } = useParams()
  const [searchParams, setSearchParams] = useSearchParams()
  const rawSource = searchParams.get("source")
  const rawAdjustment = searchParams.get("adjustment")
  const tradeDate = searchParams.get("trade_date") ?? ""
  const source = parseAnalysisSource(rawSource)
  const adjustment = parsePriceAdjustment(rawAdjustment)
  const selectedQuoteScope = `${securityCode ?? ""}|${tradeDate}`
  const [selectedQuoteState, setSelectedQuoteState] = useState({
    scope: "",
    tradeDate: "",
  })
  const selectedQuoteDate =
    selectedQuoteState.scope === selectedQuoteScope
      ? selectedQuoteState.tradeDate
      : tradeDate
  const [visiblePriceOverlays, setVisiblePriceOverlays] = useState<
    PriceOverlayKey[]
  >([
    ...DEFAULT_PRICE_OVERLAYS,
  ])
  const isDesktop = useMediaQuery("(min-width: 1024px)")

  const runQuery = useRunQuery(runId)
  const daysQuery = useRunDaysQuery(runId, runQuery.data?.status)
  const defaultTradeDate = useMemo(
    () => selectDefaultTradeDate(daysQuery.data ?? []),
    [daysQuery.data],
  )

  useEffect(() => {
    if (!runId || !securityCode) {
      return
    }
    if (rawSource && !source) {
      return
    }

    const nextTradeDate = tradeDate || defaultTradeDate
    const nextSource = source ?? DEFAULT_ANALYSIS_SOURCE
    if (!nextTradeDate) {
      return
    }
    if (
      tradeDate === nextTradeDate &&
      source === nextSource &&
      rawAdjustment === adjustment
    ) {
      return
    }

    setSearchParams(
      {
        adjustment,
        source: nextSource,
        trade_date: nextTradeDate,
      },
      { replace: true },
    )
  }, [
    adjustment,
    defaultTradeDate,
    rawAdjustment,
    rawSource,
    runId,
    securityCode,
    setSearchParams,
    source,
    tradeDate,
  ])

  const analysisQueryParams = useMemo(
    () =>
      buildSecurityAnalysisQuery({
        adjustment,
        source,
        tradeDate,
      }),
    [adjustment, source, tradeDate],
  )
  const analysisQuery = useSecurityAnalysisQuery(
    runId,
    securityCode,
    analysisQueryParams,
  )
  const analysis = analysisQuery.data
  const selectedQuote = analysis
    ? quoteForTradeDate(analysis.quote_rows, selectedQuoteDate)
    : null
  const returnPath =
    runId && source && tradeDate
      ? buildRunDetailPath({ runId, source, tradeDate })
      : "/runs"

  const setAdjustment = useCallback(
    (nextAdjustment: PriceAdjustment) => {
      if (!source || !tradeDate) {
        return
      }
      setSearchParams({
        adjustment: nextAdjustment,
        source,
        trade_date: tradeDate,
      })
    },
    [setSearchParams, source, tradeDate],
  )
  const setSelectedDate = useCallback(
    (nextTradeDate: string) => {
      setSelectedQuoteState({
        scope: selectedQuoteScope,
        tradeDate: nextTradeDate,
      })
    },
    [selectedQuoteScope],
  )

  if (!runId || !securityCode) {
    return (
      <MissingBackendState
        description="The route did not include both run_id and security_code."
        title="Security analysis not selected"
      />
    )
  }

  if (rawSource && !source) {
    return (
      <MissingBackendState
        description="The source query must be signals or pool."
        title="Invalid analysis source"
      />
    )
  }

  if (!tradeDate || !source) {
    if (daysQuery.isPending || runQuery.isPending) {
      return <TableSkeleton rows={7} />
    }
    return (
      <MissingBackendState
        description="The page needs a trade_date and source to restore the run result context."
        title="Analysis URL incomplete"
        retry={() => {
          if (defaultTradeDate) {
            setSearchParams(
              {
                adjustment,
                source: DEFAULT_ANALYSIS_SOURCE,
                trade_date: defaultTradeDate,
              },
              { replace: true },
            )
          }
        }}
      />
    )
  }

  const content = (
    <>
      <ResultRail
        adjustment={adjustment}
        currentSecurityCode={securityCode}
        runId={runId}
        source={source}
        tradeDate={tradeDate}
        resultSnapshot={analysis?.result_snapshot}
      />
      <ChartWorkspace
        adjustment={adjustment}
        analysis={analysis}
        error={analysisQuery.error}
        isError={analysisQuery.isError}
        isFetching={analysisQuery.isFetching}
        isPending={analysisQuery.isPending}
        onAdjustmentChange={setAdjustment}
        onRetry={() => void analysisQuery.refetch()}
        onSelectedDateChange={setSelectedDate}
        onVisiblePriceOverlaysChange={setVisiblePriceOverlays}
        selectedQuoteDate={selectedQuoteDate}
        visiblePriceOverlays={visiblePriceOverlays}
      />
      <IndicatorRail
        analysis={analysis}
        selectedQuote={selectedQuote}
        selectedQuoteDate={selectedQuoteDate}
      />
    </>
  )

  return (
    <div className="flex w-[calc(100vw-1.5rem)] max-w-[100rem] flex-col gap-3 self-center">
      <header className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
        <div className="min-w-0">
          <Button
            nativeButton={false}
            render={<Link to={returnPath} />}
            size="sm"
            variant="ghost"
          >
            <RacinglineIcon icon={ArrowLeft01Icon} inline="start" />
            Results
          </Button>
          <div className="mt-2 flex min-w-0 flex-col gap-1">
            <h1 className="truncate text-xl font-medium">{securityCode}</h1>
            <div className="flex flex-wrap items-center gap-2 text-sm text-muted-foreground">
              <span className="font-mono">{shortId(runId, 18)}</span>
              <Badge variant="outline">{SOURCE_LABELS[source]}</Badge>
              <Badge variant="secondary">Signal day {tradeDate}</Badge>
            </div>
          </div>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          {analysisQuery.isFetching ? (
            <Badge variant="outline">updating</Badge>
          ) : null}
          <Button
            disabled={analysisQuery.isFetching}
            onClick={() => void analysisQuery.refetch()}
            size="sm"
            variant="outline"
          >
            <RacinglineIcon icon={ArrowReloadHorizontalIcon} inline="start" />
            Refresh
          </Button>
        </div>
      </header>

      {isDesktop ? (
        <div className="grid min-h-[calc(100dvh-9rem)] gap-3 lg:grid-cols-[18rem_minmax(0,1fr)_22rem] xl:grid-cols-[20rem_minmax(0,1fr)_24rem]">
          {content}
        </div>
      ) : (
        <Tabs defaultValue="chart">
          <TabsList className="w-full">
            <TabsTrigger value="results">Results</TabsTrigger>
            <TabsTrigger value="chart">Chart</TabsTrigger>
            <TabsTrigger value="indicators">Indicators</TabsTrigger>
          </TabsList>
          <TabsContent value="results">
            <ResultRail
              adjustment={adjustment}
              currentSecurityCode={securityCode}
              runId={runId}
              source={source}
              tradeDate={tradeDate}
              resultSnapshot={analysis?.result_snapshot}
            />
          </TabsContent>
          <TabsContent value="chart">
            <ChartWorkspace
              adjustment={adjustment}
              analysis={analysis}
              error={analysisQuery.error}
              isError={analysisQuery.isError}
              isFetching={analysisQuery.isFetching}
              isPending={analysisQuery.isPending}
              onAdjustmentChange={setAdjustment}
              onRetry={() => void analysisQuery.refetch()}
              onSelectedDateChange={setSelectedDate}
              onVisiblePriceOverlaysChange={setVisiblePriceOverlays}
              selectedQuoteDate={selectedQuoteDate}
              visiblePriceOverlays={visiblePriceOverlays}
            />
          </TabsContent>
          <TabsContent value="indicators">
            <IndicatorRail
              analysis={analysis}
              selectedQuote={selectedQuote}
              selectedQuoteDate={selectedQuoteDate}
            />
          </TabsContent>
        </Tabs>
      )}
    </div>
  )
}

function ResultRail({
  adjustment,
  currentSecurityCode,
  runId,
  source,
  tradeDate,
  resultSnapshot,
}: {
  adjustment: PriceAdjustment
  currentSecurityCode: string
  runId: string
  source: AnalysisSource
  tradeDate: string
  resultSnapshot?: ResultSnapshot
}) {
  const [offset, setOffset] = useState(0)
  const [securityFilter, setSecurityFilter] = useState("")
  const query = {
    limit: PAGE_SIZE,
    offset,
    security_code: securityFilter,
    sort: source === "signals" ? "rank_asc" : "score_desc",
    trade_date: tradeDate,
  }
  const signalsQuery = useBuySignalsQuery(
    source === "signals" ? runId : undefined,
    source === "signals" ? query : undefined,
  )
  const poolQuery = usePoolMembersQuery(
    source === "pool" ? runId : undefined,
    source === "pool" ? query : undefined,
  )
  const activeQuery = source === "signals" ? signalsQuery : poolQuery
  const rows: ResultRow[] =
    source === "signals"
      ? signalsQuery.data?.items ?? []
      : poolQuery.data?.items ?? []
  const hasCurrent = rows.some((row) => row.security_code === currentSecurityCode)

  return (
    <aside className="flex min-h-0 flex-col rounded-md border bg-background lg:max-h-[calc(100dvh-9rem)]">
      <div className="flex shrink-0 flex-col gap-3 border-b p-3">
        <div className="flex items-center justify-between gap-2">
          <div className="min-w-0">
            <div className="truncate text-sm font-medium">
              {SOURCE_LABELS[source]}
            </div>
            <div className="truncate text-xs text-muted-foreground">
              {tradeDate}
            </div>
          </div>
          <Badge variant="outline">{formatCount(rows.length)} rows</Badge>
        </div>
        <Input
          onChange={(event) => {
            setSecurityFilter(event.currentTarget.value)
            setOffset(0)
          }}
          placeholder="security_code"
          value={securityFilter}
        />
        <div className="flex items-center justify-between gap-2">
          <Button
            disabled={offset === 0 || activeQuery.isFetching}
            onClick={() => setOffset(Math.max(0, offset - PAGE_SIZE))}
            size="sm"
            variant="outline"
          >
            <RacinglineIcon icon={ArrowLeft01Icon} inline="start" />
            Prev
          </Button>
          <Button
            disabled={!activeQuery.data?.has_more || activeQuery.isFetching}
            onClick={() => setOffset(offset + PAGE_SIZE)}
            size="sm"
            variant="outline"
          >
            Next
            <RacinglineIcon icon={ArrowRight01Icon} inline="end" />
          </Button>
        </div>
      </div>

      <div className="min-h-0 flex-1 overflow-y-auto p-2">
        {activeQuery.isPending ? <TableSkeleton rows={8} /> : null}
        {activeQuery.isError ? (
          <MissingBackendState
            description={`GET /rearview/runs/{run_id}/${source} did not return a usable paged response.`}
            retry={() => void activeQuery.refetch()}
            title="Result list API unavailable"
          />
        ) : null}
        {activeQuery.isSuccess && rows.length === 0 ? (
          <MissingBackendState
            description="Rearview returned no rows for this trade date and source."
            title="No result rows"
          />
        ) : null}
        {!hasCurrent && resultSnapshot ? (
          <CurrentSecuritySummary
            securityCode={currentSecurityCode}
            snapshot={resultSnapshot}
          />
        ) : null}
        <div className="flex flex-col gap-2">
          {rows.map((row) => (
            <ResultListItem
              adjustment={adjustment}
              currentSecurityCode={currentSecurityCode}
              key={`${row.trade_date}-${row.security_code}`}
              row={row}
              runId={runId}
              source={source}
              tradeDate={tradeDate}
            />
          ))}
        </div>
      </div>
    </aside>
  )
}

function ResultListItem({
  adjustment,
  currentSecurityCode,
  row,
  runId,
  source,
  tradeDate,
}: {
  adjustment: PriceAdjustment
  currentSecurityCode: string
  row: ResultRow
  runId: string
  source: AnalysisSource
  tradeDate: string
}) {
  const active = row.security_code === currentSecurityCode
  const path = buildSecurityAnalysisPath({
    adjustment,
    runId,
    securityCode: row.security_code,
    source,
    tradeDate,
  })

  return (
    <Link
      className={cn(
        "block rounded-md border p-2 outline-none transition-colors hover:bg-muted focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50",
        active ? "border-foreground bg-muted" : "border-border bg-background",
      )}
      to={path}
    >
      <div className="flex items-start justify-between gap-2">
        <div className="min-w-0">
          <div className="truncate font-mono text-sm">{row.security_code}</div>
          <div className="mt-1 flex flex-wrap gap-1">
            {rowRank(row, source) ? (
              <Badge variant="secondary">{rowRank(row, source)}</Badge>
            ) : null}
            <Badge variant="outline">score {formatScore(row.score)}</Badge>
          </div>
        </div>
        <RacinglineIcon icon={EyeIcon} />
      </div>
      <MetricPreview metrics={row.selected_metrics} />
    </Link>
  )
}

function CurrentSecuritySummary({
  securityCode,
  snapshot,
}: {
  securityCode: string
  snapshot: ResultSnapshot
}) {
  return (
    <div className="mb-2 rounded-md border bg-muted p-2">
      <div className="text-xs text-muted-foreground">Current security</div>
      <div className="truncate font-mono text-sm">{securityCode}</div>
      <div className="mt-1 flex flex-wrap gap-1">
        {snapshot.rank ? <Badge variant="secondary">rank {snapshot.rank}</Badge> : null}
        {snapshot.signal_rank ? (
          <Badge variant="secondary">signal {snapshot.signal_rank}</Badge>
        ) : null}
        <Badge variant="outline">score {formatScore(snapshot.score)}</Badge>
      </div>
    </div>
  )
}

function MetricPreview({ metrics }: { metrics: JsonRecord }) {
  const allEntries = jsonEntries(metrics)
  const prioritizedKeys = METRIC_PREVIEW_PRIORITY.filter((key) =>
    Object.prototype.hasOwnProperty.call(metrics, key),
  )
  const remainingKeys = allEntries
    .map(([key]) => key)
    .filter((key) => !prioritizedKeys.includes(key))
  const entries = [...prioritizedKeys, ...remainingKeys]
    .slice(0, 5)
    .map((key) => [key, metrics[key]] as [string, JsonValue])
  if (entries.length === 0) {
    return null
  }
  return (
    <div className="mt-2 grid gap-1 text-xs text-muted-foreground">
      {entries.map(([key, value]) => (
        <div key={key} className="grid grid-cols-[minmax(0,1fr)_auto] gap-2">
          <span className="truncate">{key}</span>
          <span className="max-w-24 truncate font-mono">
            {displayJsonValue(value)}
          </span>
        </div>
      ))}
    </div>
  )
}

function ChartWorkspace({
  adjustment,
  analysis,
  error,
  isError,
  isFetching,
  isPending,
  onAdjustmentChange,
  onRetry,
  onSelectedDateChange,
  onVisiblePriceOverlaysChange,
  selectedQuoteDate,
  visiblePriceOverlays,
}: {
  adjustment: PriceAdjustment
  analysis?: SecurityAnalysisResponse
  error: unknown
  isError: boolean
  isFetching: boolean
  isPending: boolean
  onAdjustmentChange: (adjustment: PriceAdjustment) => void
  onRetry: () => void
  onSelectedDateChange: (tradeDate: string) => void
  onVisiblePriceOverlaysChange: (keys: PriceOverlayKey[]) => void
  selectedQuoteDate: string
  visiblePriceOverlays: PriceOverlayKey[]
}) {
  const priceOverlayStatus =
    analysis?.chart.price_overlays?.status ?? analysis?.chart.ma.status
  const availablePriceOverlays = (analysis?.chart.price_overlays
    ?.available_keys ?? []) as PriceOverlayKey[]
  const overlayAvailable = priceOverlayStatus === "available"

  return (
    <main className="flex min-w-0 flex-col gap-3">
      <div className="rounded-md border bg-background">
        <div className="flex flex-col gap-3 p-3">
          <div className="flex flex-col gap-2 xl:flex-row xl:items-center xl:justify-between">
            <PriceChartToolbar
              adjustment={adjustment}
              availablePriceOverlays={availablePriceOverlays}
              disabled={!analysis}
              onAdjustmentChange={onAdjustmentChange}
              onSelectedDateChange={() => {
                if (analysis) {
                  onSelectedDateChange(analysis.trade_date)
                }
              }}
              onVisiblePriceOverlaysChange={onVisiblePriceOverlaysChange}
              selectedQuoteDate={selectedQuoteDate}
              signalDate={analysis?.trade_date}
              visiblePriceOverlays={visiblePriceOverlays}
            />
            <div className="flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
              {overlayAvailable ? null : (
                <Badge variant="secondary">Forward-adjusted overlays</Badge>
              )}
              {isFetching ? <Badge variant="outline">loading</Badge> : null}
            </div>
          </div>
          <div className="grid gap-2 text-xs sm:grid-cols-3">
            <DateChip label="Signal day" value={analysis?.trade_date ?? "-"} />
            <DateChip label="Selected day" value={selectedQuoteDate || "-"} />
            <DateChip
              label="Window"
              value={
                analysis
                  ? `${analysis.chart_window.start_date} / ${analysis.chart_window.end_date}`
                  : "-"
              }
            />
          </div>
        </div>
      </div>

      {isPending ? <ChartSkeleton /> : null}
      {isError ? (
        <ErrorState
          action={
            <Button onClick={onRetry} size="sm" variant="outline">
              Retry chart
            </Button>
          }
          error={error}
          title="Analysis API returned an error"
        />
      ) : null}
      {analysis ? (
        <SecurityAnalysisChart
          availablePriceOverlays={availablePriceOverlays}
          onSelectedDateChange={onSelectedDateChange}
          rows={analysis.chart.series}
          selectedDate={selectedQuoteDate || analysis.trade_date}
          signalDate={analysis.trade_date}
          visiblePriceOverlays={visiblePriceOverlays}
        />
      ) : null}
    </main>
  )
}

function PriceChartToolbar({
  adjustment,
  availablePriceOverlays,
  disabled,
  onAdjustmentChange,
  onSelectedDateChange,
  onVisiblePriceOverlaysChange,
  selectedQuoteDate,
  signalDate,
  visiblePriceOverlays,
}: {
  adjustment: PriceAdjustment
  availablePriceOverlays: PriceOverlayKey[]
  disabled: boolean
  onAdjustmentChange: (adjustment: PriceAdjustment) => void
  onSelectedDateChange: () => void
  onVisiblePriceOverlaysChange: (keys: PriceOverlayKey[]) => void
  selectedQuoteDate: string
  signalDate?: string
  visiblePriceOverlays: PriceOverlayKey[]
}) {
  const availableOverlaySet = new Set(availablePriceOverlays)

  return (
    <div className="min-w-0 overflow-x-auto pb-1">
      <div className="flex w-max items-center gap-2">
        <ToggleGroup
          onValueChange={(nextValue) => {
            const nextAdjustment = nextValue[0]
            if (nextAdjustment) {
              onAdjustmentChange(parsePriceAdjustment(nextAdjustment))
            }
          }}
          size="sm"
          spacing={0}
          value={[adjustment]}
          variant="outline"
        >
          {ADJUSTMENT_OPTIONS.map((option) => (
            <ToggleGroupItem
              aria-label={option.label}
              key={option.value}
              value={option.value}
            >
              {option.label}
            </ToggleGroupItem>
          ))}
        </ToggleGroup>
        <ToggleGroup
          multiple
          onValueChange={(nextValue) => {
            const nextKeys = PRICE_OVERLAY_KEYS.filter((key) =>
              nextValue.includes(key),
            )
            onVisiblePriceOverlaysChange(nextKeys)
          }}
          size="sm"
          spacing={1}
          value={visiblePriceOverlays}
          variant="outline"
        >
          {PRICE_OVERLAY_KEYS.map((key) => (
            <ToggleGroupItem
              aria-label={PRICE_OVERLAY_LABELS[key]}
              disabled={!availableOverlaySet.has(key)}
              key={key}
              value={key}
            >
              {PRICE_OVERLAY_LABELS[key]}
            </ToggleGroupItem>
          ))}
        </ToggleGroup>
        <Button
          disabled={disabled || !signalDate || selectedQuoteDate === signalDate}
          onClick={onSelectedDateChange}
          size="sm"
          variant="outline"
        >
          Signal day
        </Button>
      </div>
    </div>
  )
}

function IndicatorRail({
  analysis,
  selectedQuote,
  selectedQuoteDate,
}: {
  analysis?: SecurityAnalysisResponse
  selectedQuote: QuoteMartRow | null
  selectedQuoteDate: string
}) {
  return (
    <aside className="flex min-h-0 flex-col gap-3 rounded-md border bg-background p-3 lg:max-h-[calc(100dvh-9rem)] lg:overflow-y-auto">
      <div className="flex flex-col gap-2">
        <div className="flex items-center justify-between gap-2">
          <div className="min-w-0">
            <div className="truncate text-sm font-medium">Mart indicators</div>
            <div className="truncate text-xs text-muted-foreground">
              {analysis?.security_code ?? "-"}
            </div>
          </div>
        </div>
        <div className="grid gap-2 text-xs">
          <DateChip label="Signal day" value={analysis?.trade_date ?? "-"} />
          <DateChip label="Selected day" value={selectedQuoteDate || "-"} />
        </div>
      </div>

      <Separator />

      {!analysis ? (
        <IndicatorSkeleton />
      ) : selectedQuote ? (
        <div className="flex flex-col gap-3">
          {QUOTE_GROUPS.map((group) => (
            <QuoteGroup
              fields={group.fields}
              key={group.title}
              quote={selectedQuote}
              title={group.title}
            />
          ))}
        </div>
      ) : (
        <MissingBackendState
          description="No mart_stock_quotes_daily row exists for the selected day."
          title="No selected quote"
        />
      )}

      {analysis ? (
        <>
          <Separator />
          <RunSnapshotPanel snapshot={analysis.result_snapshot} />
        </>
      ) : null}
    </aside>
  )
}

function RunSnapshotPanel({ snapshot }: { snapshot: ResultSnapshot }) {
  return (
    <section className="flex flex-col gap-3">
      <div>
        <div className="text-sm font-medium">PostgreSQL run snapshot</div>
        <div className="text-xs text-muted-foreground">
          score_breakdown and selected_metrics
        </div>
      </div>
      <div className="grid grid-cols-2 gap-2">
        <MiniStat label="rank" value={snapshot.rank ?? snapshot.signal_rank} />
        <MiniStat label="score" value={formatScore(snapshot.score)} />
      </div>
      <JsonDetails label="selected_metrics" value={snapshot.selected_metrics} />
      {snapshot.score_breakdown ? (
        <JsonDetails label="score_breakdown" value={snapshot.score_breakdown} />
      ) : null}
      {snapshot.filter_snapshot ? (
        <JsonDetails label="filter_snapshot" value={snapshot.filter_snapshot} />
      ) : null}
    </section>
  )
}

function QuoteGroup({
  fields,
  quote,
  title,
}: {
  fields: QuoteField[]
  quote: QuoteMartRow
  title: string
}) {
  return (
    <section className="rounded-md border">
      <div className="border-b px-3 py-2 text-sm font-medium">{title}</div>
      <div className="grid gap-1 p-3">
        {fields.map((field) => (
          <QuoteFieldRow field={field} key={field.key} quote={quote} />
        ))}
      </div>
    </section>
  )
}

function QuoteFieldRow({
  field,
  quote,
}: {
  field: QuoteField
  quote: QuoteMartRow
}) {
  return (
    <div className="grid grid-cols-[minmax(0,1fr)_minmax(5rem,auto)] gap-3 text-xs">
      <span className="truncate text-muted-foreground">{field.label}</span>
      <span className="min-w-0 text-right font-mono">
        {renderQuoteValue(quote[field.key], field.kind)}
      </span>
    </div>
  )
}

function DateChip({ label, value }: { label: string; value: string }) {
  return (
    <div className="min-w-0 rounded-md bg-muted px-2 py-1">
      <div className="truncate text-muted-foreground">{label}</div>
      <div className="truncate font-mono text-xs text-foreground">{value}</div>
    </div>
  )
}

function MiniStat({
  label,
  value,
}: {
  label: string
  value: ReactNode
}) {
  return (
    <div className="min-w-0 rounded-md bg-muted p-2">
      <div className="truncate text-xs text-muted-foreground">{label}</div>
      <div className="truncate font-mono text-xs">{value ?? "-"}</div>
    </div>
  )
}

function JsonDetails({
  label,
  value,
}: {
  label: string
  value: JsonRecord | JsonValue
}) {
  return (
    <details className="rounded-md border">
      <summary className="cursor-pointer px-3 py-2 text-sm font-medium">
        {label}
      </summary>
      <pre className="max-h-72 overflow-auto border-t bg-muted p-3 text-xs whitespace-pre-wrap">
        {jsonPreview(value)}
      </pre>
    </details>
  )
}

function ChartSkeleton() {
  return (
    <div className="flex h-190 flex-col gap-3 rounded-md border bg-background p-3">
      <Skeleton className="h-112 w-full" />
      <div className="grid flex-1 gap-3 sm:grid-cols-4">
        <Skeleton className="h-full w-full" />
        <Skeleton className="h-full w-full" />
        <Skeleton className="h-full w-full" />
        <Skeleton className="h-full w-full" />
      </div>
    </div>
  )
}

function IndicatorSkeleton() {
  return (
    <div className="flex flex-col gap-3">
      {Array.from({ length: 6 }, (_, index) => (
        <Skeleton className="h-24 w-full" key={index} />
      ))}
    </div>
  )
}

function rowRank(row: ResultRow, source: AnalysisSource) {
  if (source === "signals" && "rank" in row) {
    return `rank ${row.rank}`
  }
  if ("signal_rank" in row && row.signal_rank) {
    return `signal ${row.signal_rank}`
  }
  return ""
}

function renderQuoteValue(
  value: QuoteMartRow[keyof QuoteMartRow],
  kind?: QuoteField["kind"],
) {
  if (value === undefined || value === null) {
    return "-"
  }
  if (kind === "boolean" || typeof value === "boolean") {
    return (
      <Badge variant={value ? "destructive" : "outline"}>
        {value ? "true" : "false"}
      </Badge>
    )
  }
  if (kind === "text" || kind === "date" || typeof value === "string") {
    return value
  }
  return formatScore(value)
}

function useMediaQuery(query: string) {
  const [matches, setMatches] = useState(() =>
    typeof window === "undefined" ? true : window.matchMedia(query).matches,
  )

  useEffect(() => {
    const mediaQuery = window.matchMedia(query)
    const handleChange = () => setMatches(mediaQuery.matches)
    handleChange()
    mediaQuery.addEventListener("change", handleChange)
    return () => mediaQuery.removeEventListener("change", handleChange)
  }, [query])

  return matches
}
