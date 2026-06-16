import { Link, useParams } from "react-router-dom"
import {
  ArrowLeft01Icon,
  ArrowReloadHorizontalIcon,
} from "@hugeicons/core-free-icons"

import {
  usePortfolioEventsQuery,
  usePortfolioNavQuery,
  usePortfolioOrdersQuery,
  usePortfolioPositionsQuery,
  usePortfolioRunQuery,
  usePortfolioTargetsQuery,
  usePortfolioTradesQuery,
} from "@/api/hooks"
import {
  ErrorState,
  MissingBackendState,
  TableSkeleton,
} from "@/components/racingline/data-state"
import { RacinglineIcon } from "@/components/racingline/icon"
import { StatusBadge } from "@/components/racingline/status-badge"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import {
  Card,
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
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import {
  formatCount,
  formatMoney,
  formatPct,
  formatScore,
  shortId,
} from "@/lib/format"
import type {
  PortfolioEventRecord,
  PortfolioNavRecord,
  PortfolioOrderRecord,
  PortfolioPositionRecord,
  PortfolioTargetRecord,
  PortfolioTradeRecord,
} from "@/types/rearview"

export function PortfolioDetailPage() {
  const { portfolioRunId } = useParams()
  const runQuery = usePortfolioRunQuery(portfolioRunId)
  const run = runQuery.data
  const navQuery = usePortfolioNavQuery(portfolioRunId, run?.status)
  const targetsQuery = usePortfolioTargetsQuery(portfolioRunId, { limit: 100 })
  const ordersQuery = usePortfolioOrdersQuery(portfolioRunId, { limit: 100 })
  const tradesQuery = usePortfolioTradesQuery(portfolioRunId, { limit: 100 })
  const positionsQuery = usePortfolioPositionsQuery(portfolioRunId, {
    limit: 100,
  })
  const eventsQuery = usePortfolioEventsQuery(portfolioRunId, { limit: 100 })

  function refreshAll() {
    void Promise.all([
      runQuery.refetch(),
      navQuery.refetch(),
      targetsQuery.refetch(),
      ordersQuery.refetch(),
      tradesQuery.refetch(),
      positionsQuery.refetch(),
      eventsQuery.refetch(),
    ])
  }

  if (!portfolioRunId) {
    return (
      <MissingBackendState
        description="The route did not include a portfolio_run_id."
        title="Portfolio not selected"
      />
    )
  }

  if (runQuery.isPending) {
    return <TableSkeleton rows={7} />
  }

  if (runQuery.isError) {
    return (
      <MissingBackendState
        description="GET /rearview/portfolio-runs/{portfolio_run_id} did not return a usable record."
        retry={() => void runQuery.refetch()}
        title="Portfolio API unavailable"
      />
    )
  }

  if (!run) {
    return (
      <MissingBackendState
        description="Rearview returned no portfolio run record for this id."
        title="Portfolio not found"
      />
    )
  }

  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
        <div className="min-w-0">
          <Button
            nativeButton={false}
            render={<Link to="/portfolios" />}
            size="sm"
            variant="ghost"
          >
            <RacinglineIcon icon={ArrowLeft01Icon} inline="start" />
            Portfolios
          </Button>
          <h1 className="mt-2 truncate text-xl font-medium">
            {shortId(run.portfolio_run_id, 18)}
          </h1>
          <p className="text-sm text-muted-foreground">
            {run.start_date} / {run.end_date}
          </p>
        </div>
        <div className="flex gap-2">
          <StatusBadge status={run.status} />
          <Badge variant="outline">{run.dispatch_status}</Badge>
          <Button
            disabled={runQuery.isFetching}
            onClick={refreshAll}
            size="sm"
            variant="outline"
          >
            <RacinglineIcon icon={ArrowReloadHorizontalIcon} inline="start" />
            Refresh
          </Button>
        </div>
      </div>

      {run.error_type || run.error_message ? (
        <ErrorState
          error={new Error(run.error_message ?? run.error_type ?? "failed")}
          title={run.error_type ?? "Portfolio run failed"}
        />
      ) : null}

      <Card>
        <CardHeader>
          <CardTitle>Summary</CardTitle>
          <CardDescription>
            Source run{" "}
            <Link
              className="font-mono underline-offset-4 hover:underline"
              to={`/runs/${run.source_run_id}`}
            >
              {shortId(run.source_run_id)}
            </Link>
          </CardDescription>
        </CardHeader>
        <CardContent>
          <div className="grid gap-2 sm:grid-cols-2 lg:grid-cols-4">
            <SummaryItem
              label="initial_cash"
              value={formatMoney(run.summary.initial_cash)}
            />
            <SummaryItem
              label="ending_equity"
              value={formatMoney(run.summary.ending_equity)}
            />
            <SummaryItem
              label="total_return"
              value={formatPct(run.summary.total_return)}
            />
            <SummaryItem
              label="max_drawdown"
              value={formatPct(run.summary.max_drawdown)}
            />
            <SummaryItem
              label="trade_count"
              value={formatCount(run.summary.trade_count)}
            />
            <SummaryItem
              label="total_fee"
              value={formatMoney(run.summary.total_fee)}
            />
            <SummaryItem
              label="warnings"
              value={formatCount(run.summary.warning_count)}
            />
            <SummaryItem label="price_basis" value={run.price_basis} />
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Ledger</CardTitle>
          <CardDescription>
            NAV and persisted target, order, trade, position and event rows.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <Tabs defaultValue="nav">
            <TabsList>
              <TabsTrigger value="nav">NAV</TabsTrigger>
              <TabsTrigger value="positions">Positions</TabsTrigger>
              <TabsTrigger value="trades">Trades</TabsTrigger>
              <TabsTrigger value="orders">Orders</TabsTrigger>
              <TabsTrigger value="targets">Targets</TabsTrigger>
              <TabsTrigger value="events">Events</TabsTrigger>
            </TabsList>
            <TabsContent value="nav">
              <NavTable
                rows={navQuery.data ?? []}
                pending={navQuery.isPending}
              />
            </TabsContent>
            <TabsContent value="positions">
              <PositionsTable rows={positionsQuery.data?.items ?? []} />
            </TabsContent>
            <TabsContent value="trades">
              <TradesTable rows={tradesQuery.data?.items ?? []} />
            </TabsContent>
            <TabsContent value="orders">
              <OrdersTable rows={ordersQuery.data?.items ?? []} />
            </TabsContent>
            <TabsContent value="targets">
              <TargetsTable rows={targetsQuery.data?.items ?? []} />
            </TabsContent>
            <TabsContent value="events">
              <EventsTable rows={eventsQuery.data?.items ?? []} />
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

function NavTable({
  rows,
  pending,
}: {
  rows: PortfolioNavRecord[]
  pending: boolean
}) {
  if (pending) {
    return <TableSkeleton rows={6} />
  }
  if (rows.length === 0) {
    return (
      <MissingBackendState
        description="NAV rows are empty until the worker writes results."
        title="No NAV rows"
      />
    )
  }
  return (
    <Table>
      <TableHeader>
        <TableRow>
          <TableHead>date</TableHead>
          <TableHead>nav</TableHead>
          <TableHead>equity</TableHead>
          <TableHead>cash</TableHead>
          <TableHead>positions</TableHead>
          <TableHead>drawdown</TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        {rows.map((row) => (
          <TableRow key={row.trade_date}>
            <TableCell>{row.trade_date}</TableCell>
            <TableCell>{formatScore(row.nav)}</TableCell>
            <TableCell>{formatMoney(row.total_equity)}</TableCell>
            <TableCell>{formatMoney(row.cash_balance)}</TableCell>
            <TableCell>{row.position_count}</TableCell>
            <TableCell>{formatPct(row.drawdown)}</TableCell>
          </TableRow>
        ))}
      </TableBody>
    </Table>
  )
}

function PositionsTable({ rows }: { rows: PortfolioPositionRecord[] }) {
  return (
    <SimpleTable
      empty="No position rows"
      headers={["date", "security", "quantity", "market_value", "return"]}
      rows={rows.map((row) => [
        row.trade_date,
        row.security_code,
        formatScore(row.quantity),
        formatMoney(row.market_value),
        formatPct(row.unrealized_return),
      ])}
    />
  )
}

function TradesTable({ rows }: { rows: PortfolioTradeRecord[] }) {
  return (
    <SimpleTable
      empty="No trade rows"
      headers={["date", "security", "side", "quantity", "amount", "fee"]}
      rows={rows.map((row) => [
        row.trade_date,
        row.security_code,
        row.side,
        formatScore(row.quantity),
        formatMoney(row.gross_amount),
        formatMoney(row.total_fee),
      ])}
    />
  )
}

function OrdersTable({ rows }: { rows: PortfolioOrderRecord[] }) {
  return (
    <SimpleTable
      empty="No order rows"
      headers={["date", "security", "side", "quantity", "reason", "status"]}
      rows={rows.map((row) => [
        row.execution_date,
        row.security_code,
        row.side,
        formatScore(row.order_quantity),
        row.reason,
        row.status,
      ])}
    />
  )
}

function TargetsTable({ rows }: { rows: PortfolioTargetRecord[] }) {
  return (
    <SimpleTable
      empty="No target rows"
      headers={["signal_date", "security", "rank", "weight", "amount"]}
      rows={rows.map((row) => [
        row.signal_date,
        row.security_code,
        formatCount(row.source_rank),
        formatPct(row.target_weight),
        formatMoney(row.target_amount),
      ])}
    />
  )
}

function EventsTable({ rows }: { rows: PortfolioEventRecord[] }) {
  return (
    <SimpleTable
      empty="No event rows"
      headers={["seq", "date", "security", "type", "message"]}
      rows={rows.map((row) => [
        formatCount(row.event_seq),
        row.trade_date ?? "-",
        row.security_code ?? "-",
        row.event_type,
        row.message,
      ])}
    />
  )
}

function SimpleTable({
  headers,
  rows,
  empty,
}: {
  headers: string[]
  rows: string[][]
  empty: string
}) {
  if (rows.length === 0) {
    return <MissingBackendState description={empty} title={empty} />
  }
  return (
    <Table>
      <TableHeader>
        <TableRow>
          {headers.map((header) => (
            <TableHead key={header}>{header}</TableHead>
          ))}
        </TableRow>
      </TableHeader>
      <TableBody>
        {rows.map((row, index) => (
          <TableRow key={index}>
            {row.map((cell, cellIndex) => (
              <TableCell key={cellIndex}>{cell}</TableCell>
            ))}
          </TableRow>
        ))}
      </TableBody>
    </Table>
  )
}
