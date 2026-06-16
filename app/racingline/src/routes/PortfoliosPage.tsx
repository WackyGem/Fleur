import { Link, useSearchParams } from "react-router-dom"
import {
  ArrowReloadHorizontalIcon,
  ChartLineData01Icon,
} from "@hugeicons/core-free-icons"

import { usePortfolioRunsQuery } from "@/api/hooks"
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
import { formatMoney, formatPct, shortId } from "@/lib/format"
import type { PortfolioRunRecord } from "@/types/rearview"

const EMPTY_PORTFOLIOS: PortfolioRunRecord[] = []

export function PortfoliosPage() {
  const [searchParams] = useSearchParams()
  const sourceRunId = searchParams.get("source_run_id") ?? undefined
  const query = usePortfolioRunsQuery({ limit: 50, source_run_id: sourceRunId })
  const runs = query.data?.items ?? EMPTY_PORTFOLIOS

  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
        <div className="min-w-0">
          <h1 className="truncate text-xl font-medium">Portfolios</h1>
          <p className="text-sm text-muted-foreground">
            Virtual account portfolio runs and NAV calculation status.
          </p>
        </div>
        <Button
          disabled={query.isFetching}
          onClick={() => void query.refetch()}
          size="sm"
          variant="outline"
        >
          <RacinglineIcon icon={ArrowReloadHorizontalIcon} inline="start" />
          Refresh
        </Button>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Portfolio runs</CardTitle>
          <CardDescription>
            {query.data?.has_more
              ? `${runs.length} loaded, more available`
              : `${runs.length} loaded`}
          </CardDescription>
          <CardAction>
            <Badge variant="outline">backward_adjusted</Badge>
          </CardAction>
        </CardHeader>
        <CardContent>
          {query.isPending ? <TableSkeleton rows={7} /> : null}
          {query.isError ? (
            <ErrorState
              error={query.error}
              title="Portfolio runs API returned an error"
            />
          ) : null}
          {query.isSuccess && runs.length === 0 ? (
            <MissingBackendState
              description="Create a portfolio run from a succeeded screening run."
              title="No portfolio runs"
            />
          ) : null}
          {runs.length > 0 ? <PortfolioRunsTable runs={runs} /> : null}
        </CardContent>
      </Card>
    </div>
  )
}

function PortfolioRunsTable({ runs }: { runs: PortfolioRunRecord[] }) {
  return (
    <Table>
      <TableHeader>
        <TableRow>
          <TableHead>portfolio_run_id</TableHead>
          <TableHead>source_run</TableHead>
          <TableHead>range</TableHead>
          <TableHead>status</TableHead>
          <TableHead>dispatch</TableHead>
          <TableHead>nav</TableHead>
          <TableHead>return</TableHead>
          <TableHead>fee</TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        {runs.map((run) => (
          <TableRow key={run.portfolio_run_id}>
            <TableCell>
              <Button
                nativeButton={false}
                render={<Link to={`/portfolios/${run.portfolio_run_id}`} />}
                size="sm"
                variant="ghost"
              >
                <RacinglineIcon icon={ChartLineData01Icon} inline="start" />
                {shortId(run.portfolio_run_id)}
              </Button>
            </TableCell>
            <TableCell>
              <Link
                className="font-mono text-xs underline-offset-4 hover:underline"
                to={`/runs/${run.source_run_id}`}
              >
                {shortId(run.source_run_id)}
              </Link>
            </TableCell>
            <TableCell>
              {run.start_date} / {run.end_date}
            </TableCell>
            <TableCell>
              <StatusBadge status={run.status} />
            </TableCell>
            <TableCell>
              <Badge variant="outline">{run.dispatch_status}</Badge>
            </TableCell>
            <TableCell>{formatMoney(run.summary?.ending_equity)}</TableCell>
            <TableCell>{formatPct(run.summary?.total_return)}</TableCell>
            <TableCell>{formatMoney(run.summary?.total_fee)}</TableCell>
          </TableRow>
        ))}
      </TableBody>
    </Table>
  )
}
