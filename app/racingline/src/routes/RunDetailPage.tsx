import { useEffect, useMemo, useState } from "react"
import { Link, useParams, useSearchParams } from "react-router-dom"
import {
  ArrowLeft01Icon,
  ArrowReloadHorizontalIcon,
} from "@hugeicons/core-free-icons"

import {
  useRunChunksQuery,
  useRunDaysQuery,
  useRunQuery,
} from "@/api/hooks"
import {
  ErrorState,
  MissingBackendState,
  TableSkeleton,
} from "@/components/racingline/data-state"
import { FilterSelect } from "@/components/racingline/filter-select"
import { RacinglineIcon } from "@/components/racingline/icon"
import { StatusBadge } from "@/components/racingline/status-badge"
import { RunProgressChart } from "@/features/runs/components/run-progress-chart"
import { PoolTab, SignalsTab } from "@/features/runs/components/run-results"
import {
  formatCount,
  formatScore,
  selectDefaultTradeDate,
  shortId,
} from "@/lib/format"
import { isFailureStatus, isRunActiveStatus } from "@/lib/status"
import { cn } from "@/lib/utils"
import { useWorkbenchStore } from "@/store/workbench"
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import {
  Card,
  CardAction,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card"
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table"
import {
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
} from "@/components/ui/tabs"
import type { RunChunkRecord, RunDayRecord } from "@/types/rearview"

const EMPTY_DAYS: RunDayRecord[] = []

export function RunDetailPage() {
  const { runId } = useParams()
  const [searchParams, setSearchParams] = useSearchParams()
  const initialSource = searchParams.get("source")
  const returnTradeDate = searchParams.get("trade_date") ?? ""
  const [tab, setTab] = useState(initialSource === "pool" ? "pool" : "signals")
  const selectedTradeDate = useWorkbenchStore((state) =>
    runId ? state.selectedTradeDateByRun[runId] : "",
  )
  const setSelectedTradeDate = useWorkbenchStore(
    (state) => state.setSelectedTradeDate,
  )

  const runQuery = useRunQuery(runId)
  const run = runQuery.data
  const chunksQuery = useRunChunksQuery(runId, run?.status)
  const daysQuery = useRunDaysQuery(runId, run?.status)
  const days = daysQuery.data ?? EMPTY_DAYS

  const defaultTradeDate = useMemo(() => selectDefaultTradeDate(days), [days])
  const tradeDate = selectedTradeDate || defaultTradeDate
  const tradeDateOptions = useMemo(
    () => [
      { label: "Select trade date", value: null },
      ...days
        .slice()
        .sort((left, right) => right.trade_date.localeCompare(left.trade_date))
        .map((day) => ({
          label: `${day.trade_date} (${day.status})`,
          value: day.trade_date,
        })),
    ],
    [days],
  )

  useEffect(() => {
    if (runId && returnTradeDate && selectedTradeDate !== returnTradeDate) {
      setSelectedTradeDate(runId, returnTradeDate)
      return
    }
    if (runId && defaultTradeDate && !selectedTradeDate) {
      setSelectedTradeDate(runId, defaultTradeDate)
    }
  }, [
    defaultTradeDate,
    returnTradeDate,
    runId,
    selectedTradeDate,
    setSelectedTradeDate,
  ])

  function selectTradeDate(nextTradeDate: string) {
    if (!runId || !nextTradeDate) {
      return
    }

    setSelectedTradeDate(runId, nextTradeDate)
    setSearchParams(
      (current) => {
        const next = new URLSearchParams(current)
        next.set("trade_date", nextTradeDate)
        if (!next.get("source")) {
          next.set("source", tab === "pool" ? "pool" : "signals")
        }
        return next
      },
      { replace: false },
    )
  }

  function refreshAll() {
    void Promise.all([
      runQuery.refetch(),
      chunksQuery.refetch(),
      daysQuery.refetch(),
    ])
  }

  if (!runId) {
    return (
      <MissingBackendState
        description="The route did not include a run_id."
        title="Run not selected"
      />
    )
  }

  if (runQuery.isPending) {
    return <TableSkeleton rows={7} />
  }

  if (runQuery.isError) {
    return (
      <MissingBackendState
        description="GET /rearview/runs/{run_id} did not return a usable run record."
        retry={() => void runQuery.refetch()}
        title="Run detail API unavailable"
      />
    )
  }

  if (!run) {
    return (
      <MissingBackendState
        description="Rearview returned no run record for this run_id."
        title="Run not found"
      />
    )
  }

  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
        <div className="min-w-0">
          <Button
            nativeButton={false}
            render={<Link to="/runs" />}
            size="sm"
            variant="ghost"
          >
            <RacinglineIcon icon={ArrowLeft01Icon} inline="start" />
            Runs
          </Button>
          <h1 className="mt-2 truncate text-xl font-medium">
            {shortId(run.run_id, 18)}
          </h1>
          <p className="text-sm text-muted-foreground">
            {run.start_date} / {run.end_date}
          </p>
        </div>
        <div className="flex gap-2">
          <StatusBadge status={run.status} />
          <Button
            disabled={
              runQuery.isFetching ||
              chunksQuery.isFetching ||
              daysQuery.isFetching
            }
            onClick={refreshAll}
            size="sm"
            variant="outline"
          >
            <RacinglineIcon icon={ArrowReloadHorizontalIcon} inline="start" />
            Refresh
          </Button>
        </div>
      </div>

      {isFailureStatus(run.status) ? (
        <Alert variant="destructive">
          <AlertTitle>{run.error_type ?? "Run failed"}</AlertTitle>
          <AlertDescription>
            {run.error_message ?? "Rearview did not return error detail."}
          </AlertDescription>
        </Alert>
      ) : null}

      <div className="grid gap-3 lg:grid-cols-[1fr_20rem]">
        <Card>
          <CardHeader>
            <CardTitle>Run summary</CardTitle>
            <CardDescription>
              {isRunActiveStatus(run.status)
                ? "Polling run, chunk and day status every 3 seconds."
                : "Terminal runs keep manual refresh available."}
            </CardDescription>
          </CardHeader>
          <CardContent>
            <div className="grid gap-2 sm:grid-cols-2 lg:grid-cols-4">
              <SummaryItem label="rule_version_id" value={run.rule_version_id} />
              <SummaryItem label="rule_hash" value={run.rule_hash} />
              <SummaryItem
                label="compiled_sql_hash"
                value={run.compiled_sql_hash ?? "-"}
              />
              <SummaryItem label="top_n" value={String(run.top_n)} />
              <SummaryItem
                label="pool_count"
                value={formatCount(run.summary?.pool_count)}
              />
              <SummaryItem
                label="signal_count"
                value={formatCount(run.summary?.signal_count)}
              />
              <SummaryItem
                label="day_count"
                value={formatCount(run.summary?.day_count)}
              />
              <SummaryItem label="status" value={run.status} />
            </div>
          </CardContent>
        </Card>
        <Card>
          <CardHeader>
            <CardTitle>Signals by day</CardTitle>
            <CardDescription>Runtime daily signal_count snapshot.</CardDescription>
          </CardHeader>
          <CardContent>
            <RunProgressChart days={days} />
          </CardContent>
        </Card>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Results</CardTitle>
          <CardDescription>Selected trade date: {tradeDate || "-"}</CardDescription>
          <CardAction>
            <FilterSelect
              className="w-56"
              onValueChange={(value) => {
                if (value) {
                  selectTradeDate(value)
                }
              }}
              options={tradeDateOptions}
              value={tradeDate}
            />
          </CardAction>
        </CardHeader>
        <CardContent>
          {daysQuery.isError ? (
            <ErrorState
              error={daysQuery.error}
              title="Run days API returned an error"
            />
          ) : null}
          <Tabs value={tab} onValueChange={(value) => setTab(String(value))}>
            <TabsList>
              <TabsTrigger value="signals">Buy signals</TabsTrigger>
              <TabsTrigger value="pool">Pool</TabsTrigger>
              <TabsTrigger value="days">Run days</TabsTrigger>
              <TabsTrigger value="chunks">Chunks</TabsTrigger>
            </TabsList>
            <TabsContent value="signals">
              {tradeDate ? (
                <SignalsTab runId={runId} tradeDate={tradeDate} />
              ) : (
                <MissingBackendState
                  description="No successful trade date is available for this run."
                  title="No trade date"
                />
              )}
            </TabsContent>
            <TabsContent value="pool">
              {tradeDate ? (
                <PoolTab runId={runId} tradeDate={tradeDate} />
              ) : (
                <MissingBackendState
                  description="No successful trade date is available for this run."
                  title="No trade date"
                />
              )}
            </TabsContent>
            <TabsContent value="days">
              {daysQuery.isPending ? <TableSkeleton /> : null}
              <RunDaysTable
                days={days}
                selectedTradeDate={tradeDate}
                onSelect={selectTradeDate}
              />
            </TabsContent>
            <TabsContent value="chunks">
              {chunksQuery.isPending ? <TableSkeleton /> : null}
              {chunksQuery.isError ? (
                <ErrorState
                  error={chunksQuery.error}
                  title="Run chunks API returned an error"
                />
              ) : null}
              <RunChunksTable chunks={chunksQuery.data ?? []} />
            </TabsContent>
          </Tabs>
        </CardContent>
      </Card>
    </div>
  )
}

function SummaryItem({ label, value }: { label: string; value: string }) {
  return (
    <div className="min-w-0 rounded-lg bg-muted p-2">
      <div className="truncate text-xs text-muted-foreground">{label}</div>
      <div className="truncate font-mono text-xs">{value}</div>
    </div>
  )
}

function RunDaysTable({
  days,
  selectedTradeDate,
  onSelect,
}: {
  days: RunDayRecord[]
  selectedTradeDate: string
  onSelect: (tradeDate: string) => void
}) {
  if (days.length === 0) {
    return (
      <MissingBackendState
        description="Rearview returned no day records for this run."
        title="No run days"
      />
    )
  }

  return (
    <Table>
      <TableHeader>
        <TableRow>
          <TableHead>trade_date</TableHead>
          <TableHead>status</TableHead>
          <TableHead>universe</TableHead>
          <TableHead>pool</TableHead>
          <TableHead>signals</TableHead>
          <TableHead>error</TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        {days.map((day) => (
          <TableRow
            key={day.trade_date}
            className={cn(
              selectedTradeDate === day.trade_date ? "bg-muted/50" : "",
            )}
          >
            <TableCell>
              <Button
                onClick={() => onSelect(day.trade_date)}
                size="sm"
                variant="ghost"
              >
                {day.trade_date}
              </Button>
            </TableCell>
            <TableCell>
              <StatusBadge status={day.status} />
            </TableCell>
            <TableCell>{formatCount(day.universe_count)}</TableCell>
            <TableCell>{formatCount(day.pool_count)}</TableCell>
            <TableCell>{formatCount(day.signal_count)}</TableCell>
            <TableCell>
              {day.error_type || day.error_message ? (
                <Badge variant="destructive">
                  {day.error_type ?? day.error_message}
                </Badge>
              ) : (
                <Badge variant="outline">-</Badge>
              )}
            </TableCell>
          </TableRow>
        ))}
      </TableBody>
    </Table>
  )
}

function RunChunksTable({ chunks }: { chunks: RunChunkRecord[] }) {
  if (chunks.length === 0) {
    return (
      <MissingBackendState
        description="Rearview returned no chunk records for this run."
        title="No chunks"
      />
    )
  }

  return (
    <Table>
      <TableHeader>
        <TableRow>
          <TableHead>chunk</TableHead>
          <TableHead>range</TableHead>
          <TableHead>status</TableHead>
          <TableHead>query_id</TableHead>
          <TableHead>elapsed_ms</TableHead>
          <TableHead>error</TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        {chunks.map((chunk) => (
          <TableRow key={chunk.chunk_no}>
            <TableCell>{chunk.chunk_no}</TableCell>
            <TableCell>
              {chunk.start_date} / {chunk.end_date}
            </TableCell>
            <TableCell>
              <StatusBadge status={chunk.status} />
            </TableCell>
            <TableCell className="max-w-44 truncate font-mono text-xs">
              {chunk.clickhouse_query_id ?? "-"}
            </TableCell>
            <TableCell>{formatScore(chunk.elapsed_ms)}</TableCell>
            <TableCell>
              {chunk.error_type || chunk.error_message ? (
                <Badge variant="destructive">
                  {chunk.error_type ?? chunk.error_message}
                </Badge>
              ) : (
                <Badge variant="outline">-</Badge>
              )}
            </TableCell>
          </TableRow>
        ))}
      </TableBody>
    </Table>
  )
}
