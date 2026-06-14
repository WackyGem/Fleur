import { useMemo, useState } from "react"
import {
  ArrowLeft01Icon,
  ArrowRight01Icon,
  EyeIcon,
} from "@hugeicons/core-free-icons"

import {
  useBuySignalsQuery,
  usePoolMembersQuery,
} from "@/api/hooks"
import { MissingBackendState, TableSkeleton } from "@/components/racingline/data-state"
import { RacinglineIcon } from "@/components/racingline/icon"
import { SignalDetailSheet } from "@/features/runs/components/signal-detail-sheet"
import {
  displayJsonValue,
  formatScore,
  metricColumns,
} from "@/lib/format"
import { useWorkbenchStore } from "@/store/workbench"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table"
import type { BuySignalRecord } from "@/types/rearview"

type ResultTabProps = {
  runId: string
  tradeDate: string
}

const PAGE_SIZE = 50
const EMPTY_SIGNALS: BuySignalRecord[] = []
const EMPTY_POOL: never[] = []

export function SignalsTab({ runId, tradeDate }: ResultTabProps) {
  const [offset, setOffset] = useState(0)
  const [securityCode, setSecurityCode] = useState("")
  const [selectedSignal, setSelectedSignal] = useState<BuySignalRecord | null>(
    null,
  )
  const signalDetailOpen = useWorkbenchStore((state) => state.signalDetailOpen)
  const setSignalDetailOpen = useWorkbenchStore(
    (state) => state.setSignalDetailOpen,
  )

  const query = useBuySignalsQuery(runId, {
    limit: PAGE_SIZE,
    offset,
    security_code: securityCode,
    sort: "rank_asc",
    trade_date: tradeDate,
  })
  const rows = query.data?.items ?? EMPTY_SIGNALS
  const columns = useMemo(() => metricColumns(rows), [rows])

  return (
    <div className="flex flex-col gap-3">
      <ResultToolbar
        offset={offset}
        onOffsetChange={setOffset}
        onSecurityCodeChange={(value) => {
          setSecurityCode(value)
          setOffset(0)
        }}
        securityCode={securityCode}
        hasMore={query.data?.has_more ?? false}
        isFetching={query.isFetching}
      />
      {query.isPending ? <TableSkeleton /> : null}
      {query.isError ? (
        <MissingBackendState
          description="GET /rearview/runs/{run_id}/signals did not return a usable paged response."
          retry={() => void query.refetch()}
          title="Signals API unavailable"
        />
      ) : null}
      {query.isSuccess && rows.length === 0 ? (
        <MissingBackendState
          description="Rearview returned no buy signals for the selected trade date."
          title="No buy signals"
        />
      ) : null}
      {rows.length > 0 ? (
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>rank</TableHead>
              <TableHead>security_code</TableHead>
              <TableHead>score</TableHead>
              {columns.map((column) => (
                <TableHead key={column}>{column}</TableHead>
              ))}
              <TableHead className="text-right">detail</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {rows.map((row) => (
              <TableRow key={`${row.trade_date}-${row.security_code}`}>
                <TableCell>{row.rank}</TableCell>
                <TableCell className="font-mono text-xs">
                  {row.security_code}
                </TableCell>
                <TableCell>{formatScore(row.score)}</TableCell>
                {columns.map((column) => (
                  <TableCell key={column} className="max-w-44 truncate">
                    {displayJsonValue(row.selected_metrics[column])}
                  </TableCell>
                ))}
                <TableCell>
                  <div className="flex justify-end">
                    <Button
                      onClick={() => {
                        setSelectedSignal(row)
                        setSignalDetailOpen(true)
                      }}
                      size="sm"
                      variant="outline"
                    >
                      <RacinglineIcon icon={EyeIcon} inline="start" />
                      Open
                    </Button>
                  </div>
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      ) : null}
      <SignalDetailSheet
        onOpenChange={setSignalDetailOpen}
        open={signalDetailOpen}
        signal={selectedSignal}
      />
    </div>
  )
}

export function PoolTab({ runId, tradeDate }: ResultTabProps) {
  const [offset, setOffset] = useState(0)
  const [securityCode, setSecurityCode] = useState("")
  const query = usePoolMembersQuery(runId, {
    limit: PAGE_SIZE,
    offset,
    security_code: securityCode,
    sort: "score_desc",
    trade_date: tradeDate,
  })
  const rows = query.data?.items ?? EMPTY_POOL
  const columns = useMemo(() => metricColumns(rows), [rows])

  return (
    <div className="flex flex-col gap-3">
      <ResultToolbar
        offset={offset}
        onOffsetChange={setOffset}
        onSecurityCodeChange={(value) => {
          setSecurityCode(value)
          setOffset(0)
        }}
        securityCode={securityCode}
        hasMore={query.data?.has_more ?? false}
        isFetching={query.isFetching}
      />
      {query.isPending ? <TableSkeleton /> : null}
      {query.isError ? (
        <MissingBackendState
          description="GET /rearview/runs/{run_id}/pool did not return a usable paged response."
          retry={() => void query.refetch()}
          title="Pool API unavailable"
        />
      ) : null}
      {query.isSuccess && rows.length === 0 ? (
        <MissingBackendState
          description="Rearview returned no pool members for the selected trade date."
          title="No pool members"
        />
      ) : null}
      {rows.length > 0 ? (
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>security_code</TableHead>
              <TableHead>score</TableHead>
              <TableHead>signal_rank</TableHead>
              {columns.map((column) => (
                <TableHead key={column}>{column}</TableHead>
              ))}
            </TableRow>
          </TableHeader>
          <TableBody>
            {rows.map((row) => (
              <TableRow key={`${row.trade_date}-${row.security_code}`}>
                <TableCell className="font-mono text-xs">
                  {row.security_code}
                </TableCell>
                <TableCell>{formatScore(row.score)}</TableCell>
                <TableCell>{row.signal_rank ?? "-"}</TableCell>
                {columns.map((column) => (
                  <TableCell key={column} className="max-w-44 truncate">
                    {displayJsonValue(row.selected_metrics[column])}
                  </TableCell>
                ))}
              </TableRow>
            ))}
          </TableBody>
        </Table>
      ) : null}
    </div>
  )
}

function ResultToolbar({
  hasMore,
  isFetching,
  offset,
  securityCode,
  onOffsetChange,
  onSecurityCodeChange,
}: {
  hasMore: boolean
  isFetching: boolean
  offset: number
  securityCode: string
  onOffsetChange: (offset: number) => void
  onSecurityCodeChange: (value: string) => void
}) {
  return (
    <div className="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
      <Input
        className="sm:max-w-56"
        onChange={(event) => onSecurityCodeChange(event.currentTarget.value)}
        placeholder="security_code"
        value={securityCode}
      />
      <div className="flex items-center justify-end gap-2">
        <Button
          disabled={offset === 0 || isFetching}
          onClick={() => onOffsetChange(Math.max(0, offset - PAGE_SIZE))}
          size="sm"
          variant="outline"
        >
          <RacinglineIcon icon={ArrowLeft01Icon} inline="start" />
          Prev
        </Button>
        <Button
          disabled={!hasMore || isFetching}
          onClick={() => onOffsetChange(offset + PAGE_SIZE)}
          size="sm"
          variant="outline"
        >
          Next
          <RacinglineIcon icon={ArrowRight01Icon} inline="end" />
        </Button>
      </div>
    </div>
  )
}
