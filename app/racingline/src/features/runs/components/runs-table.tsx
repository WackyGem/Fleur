import {
  AlertCircleIcon,
  ArrowRight01Icon,
  EyeIcon,
} from "@hugeicons/core-free-icons"
import { Link } from "react-router-dom"

import { CopyButton } from "@/components/racingline/copy-button"
import { RacinglineIcon } from "@/components/racingline/icon"
import { StatusBadge } from "@/components/racingline/status-badge"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog"
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table"
import { formatCount, shortId } from "@/lib/format"
import type { RunRecord } from "@/types/rearview"

type RunsTableProps = {
  runs: RunRecord[]
}

export function RunsTable({ runs }: RunsTableProps) {
  return (
    <Table>
      <TableHeader>
        <TableRow>
          <TableHead>run_id</TableHead>
          <TableHead>rule set</TableHead>
          <TableHead>version</TableHead>
          <TableHead>hash</TableHead>
          <TableHead>range</TableHead>
          <TableHead>top_n</TableHead>
          <TableHead>status</TableHead>
          <TableHead>pool</TableHead>
          <TableHead>signals</TableHead>
          <TableHead>error</TableHead>
          <TableHead className="text-right">actions</TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        {runs.map((run) => (
          <TableRow key={run.run_id}>
            <TableCell>
              <div className="flex max-w-36 items-center gap-1">
                <span className="truncate font-mono text-xs">
                  {shortId(run.run_id, 12)}
                </span>
                <CopyButton label="Copy run_id" value={run.run_id} />
              </div>
            </TableCell>
            <TableCell>
              <span className="block max-w-40 truncate">
                {run.rule_set_name ?? run.rule_set_id ?? "-"}
              </span>
            </TableCell>
            <TableCell>
              <span className="font-mono text-xs">
                {shortId(run.rule_version_id, 10)}
              </span>
            </TableCell>
            <TableCell>
              <span className="font-mono text-xs">{shortId(run.rule_hash)}</span>
            </TableCell>
            <TableCell>
              <span className="text-xs">
                {run.start_date} / {run.end_date}
              </span>
            </TableCell>
            <TableCell>{run.top_n}</TableCell>
            <TableCell>
              <StatusBadge status={run.status} />
            </TableCell>
            <TableCell>{formatCount(run.summary?.pool_count)}</TableCell>
            <TableCell>{formatCount(run.summary?.signal_count)}</TableCell>
            <TableCell>
              {run.error_type || run.error_message ? (
                <RunErrorDialog run={run} />
              ) : (
                <Badge variant="outline">-</Badge>
              )}
            </TableCell>
            <TableCell>
              <div className="flex justify-end gap-1">
                <Button
                  nativeButton={false}
                  render={<Link to={`/runs/${run.run_id}`} />}
                  size="sm"
                  variant="outline"
                >
                  <RacinglineIcon icon={EyeIcon} inline="start" />
                  Open
                  <RacinglineIcon icon={ArrowRight01Icon} inline="end" />
                </Button>
              </div>
            </TableCell>
          </TableRow>
        ))}
      </TableBody>
    </Table>
  )
}

function RunErrorDialog({ run }: { run: RunRecord }) {
  return (
    <Dialog>
      <DialogTrigger render={<Button size="sm" variant="destructive" />}>
        <RacinglineIcon icon={AlertCircleIcon} inline="start" />
        Error
      </DialogTrigger>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Run error</DialogTitle>
          <DialogDescription>
            {run.error_type ?? "run_error"} on {shortId(run.run_id, 12)}
          </DialogDescription>
        </DialogHeader>
        <pre className="max-h-64 overflow-auto rounded-lg bg-muted p-3 text-xs whitespace-pre-wrap">
          {run.error_message ?? run.error_type ?? "No error detail returned."}
        </pre>
      </DialogContent>
    </Dialog>
  )
}
