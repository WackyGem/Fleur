import { AddToListIcon } from "@hugeicons/core-free-icons"

import { CopyButton } from "@/components/racingline/copy-button"
import { RacinglineIcon } from "@/components/racingline/icon"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table"
import { splitCsv } from "@/store/workbench"
import type { MetricDefinition } from "@/types/rearview"

type MetricsTableProps = {
  metrics: MetricDefinition[]
  outputMetrics: string
  onOutputMetricsChange: (value: string) => void
}

export function MetricsTable({
  metrics,
  outputMetrics,
  onOutputMetricsChange,
}: MetricsTableProps) {
  const currentOutputMetrics = splitCsv(outputMetrics)

  function addOutputMetric(metricName: string) {
    if (currentOutputMetrics.includes(metricName)) {
      return
    }
    onOutputMetricsChange([...currentOutputMetrics, metricName].join(", "))
  }

  return (
    <Table>
      <TableHeader>
        <TableRow>
          <TableHead>logical_metric</TableHead>
          <TableHead>mart</TableHead>
          <TableHead>column</TableHead>
          <TableHead>kind</TableHead>
          <TableHead>filter</TableHead>
          <TableHead>scoring</TableHead>
          <TableHead>ops</TableHead>
          <TableHead>null_policy</TableHead>
          <TableHead>default</TableHead>
          <TableHead>description</TableHead>
          <TableHead className="text-right">actions</TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        {metrics.map((metric) => (
          <TableRow key={metric.logical_metric}>
            <TableCell>
              <div className="flex max-w-52 items-center gap-1">
                <span className="truncate font-mono text-xs">
                  {metric.logical_metric}
                </span>
                <CopyButton
                  label="Copy metric name"
                  value={metric.logical_metric}
                />
              </div>
            </TableCell>
            <TableCell>
              <span className="block max-w-44 truncate">
                {metric.mart_database}.{metric.mart_table}
              </span>
            </TableCell>
            <TableCell className="font-mono text-xs">
              {metric.column_name}
            </TableCell>
            <TableCell>
              <Badge variant="outline">{metric.value_kind}</Badge>
            </TableCell>
            <TableCell>
              <Badge variant={metric.allow_filter ? "secondary" : "outline"}>
                {metric.allow_filter ? "yes" : "no"}
              </Badge>
            </TableCell>
            <TableCell>
              <Badge variant={metric.allow_scoring ? "secondary" : "outline"}>
                {metric.allow_scoring ? "yes" : "no"}
              </Badge>
            </TableCell>
            <TableCell>
              <span className="block max-w-44 truncate">
                {metric.allowed_ops.join(", ") || "-"}
              </span>
            </TableCell>
            <TableCell>{metric.null_policy}</TableCell>
            <TableCell>
              <Badge variant={metric.default_output ? "secondary" : "outline"}>
                {metric.default_output ? "yes" : "no"}
              </Badge>
            </TableCell>
            <TableCell>
              <span className="block max-w-80 truncate">
                {metric.description ?? "-"}
              </span>
            </TableCell>
            <TableCell>
              <div className="flex justify-end">
                <Button
                  disabled={currentOutputMetrics.includes(metric.logical_metric)}
                  onClick={() => addOutputMetric(metric.logical_metric)}
                  size="sm"
                  variant="outline"
                >
                  <RacinglineIcon icon={AddToListIcon} inline="start" />
                  Output
                </Button>
              </div>
            </TableCell>
          </TableRow>
        ))}
      </TableBody>
    </Table>
  )
}
