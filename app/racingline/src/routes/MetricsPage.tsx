import { useMemo, useState } from "react"
import { ArrowReloadHorizontalIcon } from "@hugeicons/core-free-icons"

import { useMetricsQuery } from "@/api/hooks"
import { MissingBackendState, TableSkeleton } from "@/components/racingline/data-state"
import { FilterSelect } from "@/components/racingline/filter-select"
import { RacinglineIcon } from "@/components/racingline/icon"
import { MetricsTable } from "@/features/metrics/components/metrics-table"
import { useWorkbenchStore } from "@/store/workbench"
import { Button } from "@/components/ui/button"
import {
  Card,
  CardAction,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card"
import { Field, FieldGroup, FieldLabel } from "@/components/ui/field"
import { Input } from "@/components/ui/input"
import type { MetricsQuery } from "@/types/rearview"
import type { MetricDefinition } from "@/types/rearview"

const valueKindOptions = [
  { label: "All kinds", value: null },
  { label: "numeric", value: "numeric" },
  { label: "integer", value: "integer" },
  { label: "boolean", value: "boolean" },
  { label: "string", value: "string" },
  { label: "date", value: "date" },
]

const booleanOptions = [
  { label: "Any", value: null },
  { label: "Yes", value: "true" },
  { label: "No", value: "false" },
]

const EMPTY_METRICS: MetricDefinition[] = []

export function MetricsPage() {
  const [filters, setFilters] = useState<
    MetricsQuery & { allowFilterText?: string; allowScoringText?: string }
  >({})
  const draft = useWorkbenchStore((state) => state.draft)
  const setDraft = useWorkbenchStore((state) => state.setDraft)

  const queryFilters = useMemo<MetricsQuery>(
    () => ({
      allow_filter:
        filters.allowFilterText === undefined || filters.allowFilterText === ""
          ? undefined
          : filters.allowFilterText === "true",
      allow_scoring:
        filters.allowScoringText === undefined ||
        filters.allowScoringText === ""
          ? undefined
          : filters.allowScoringText === "true",
      keyword: filters.keyword,
      mart_table: filters.mart_table,
      value_kind: filters.value_kind,
    }),
    [filters],
  )
  const metricsQuery = useMetricsQuery(queryFilters)
  const metrics = metricsQuery.data ?? EMPTY_METRICS
  const martOptions = useMemo(
    () => [
      { label: "All marts", value: null },
      ...Array.from(new Set(metrics.map((metric) => metric.mart_table)))
        .sort()
        .map((martTable) => ({ label: martTable, value: martTable })),
    ],
    [metrics],
  )

  function patchFilters(
    patch: Partial<
      MetricsQuery & { allowFilterText?: string; allowScoringText?: string }
    >,
  ) {
    setFilters((current) => ({ ...current, ...patch }))
  }

  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
        <div>
          <h1 className="text-xl font-medium">Metrics</h1>
          <p className="text-sm text-muted-foreground">
            Rearview metric catalog allowlist.
          </p>
        </div>
        <Button
          disabled={metricsQuery.isFetching}
          onClick={() => void metricsQuery.refetch()}
          size="sm"
          variant="outline"
        >
          <RacinglineIcon icon={ArrowReloadHorizontalIcon} inline="start" />
          Refresh
        </Button>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Metric filters</CardTitle>
          <CardDescription>{metrics.length} metrics loaded</CardDescription>
          <CardAction>
            <Button onClick={() => setFilters({})} size="sm" variant="ghost">
              Reset
            </Button>
          </CardAction>
        </CardHeader>
        <CardContent>
          <FieldGroup className="grid gap-3 sm:grid-cols-2 lg:grid-cols-5">
            <Field>
              <FieldLabel>Mart table</FieldLabel>
              <FilterSelect
                className="w-full"
                onValueChange={(mart_table) =>
                  patchFilters({ mart_table: mart_table || undefined })
                }
                options={martOptions}
                value={filters.mart_table ?? ""}
              />
            </Field>
            <Field>
              <FieldLabel>Value kind</FieldLabel>
              <FilterSelect
                className="w-full"
                onValueChange={(value_kind) =>
                  patchFilters({ value_kind: value_kind || undefined })
                }
                options={valueKindOptions}
                value={filters.value_kind ?? ""}
              />
            </Field>
            <Field>
              <FieldLabel>Allow filter</FieldLabel>
              <FilterSelect
                className="w-full"
                onValueChange={(allowFilterText) =>
                  patchFilters({ allowFilterText })
                }
                options={booleanOptions}
                value={filters.allowFilterText ?? ""}
              />
            </Field>
            <Field>
              <FieldLabel>Allow scoring</FieldLabel>
              <FilterSelect
                className="w-full"
                onValueChange={(allowScoringText) =>
                  patchFilters({ allowScoringText })
                }
                options={booleanOptions}
                value={filters.allowScoringText ?? ""}
              />
            </Field>
            <Field>
              <FieldLabel>Keyword</FieldLabel>
              <Input
                onChange={(event) =>
                  patchFilters({ keyword: event.currentTarget.value })
                }
                value={filters.keyword ?? ""}
              />
            </Field>
          </FieldGroup>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Catalog</CardTitle>
          <CardDescription>Only metrics returned by Rearview.</CardDescription>
        </CardHeader>
        <CardContent>
          {metricsQuery.isPending ? <TableSkeleton /> : null}
          {metricsQuery.isError ? (
            <MissingBackendState
              description="GET /rearview/metrics did not return a usable catalog response."
              retry={() => void metricsQuery.refetch()}
              title="Metric catalog API unavailable"
            />
          ) : null}
          {metricsQuery.isSuccess && metrics.length === 0 ? (
            <MissingBackendState
              description="Rearview returned no metrics for the current filters."
              title="No metrics"
            />
          ) : null}
          {metrics.length > 0 ? (
            <MetricsTable
              metrics={metrics}
              onOutputMetricsChange={(outputMetrics) =>
                setDraft({ outputMetrics })
              }
              outputMetrics={draft.outputMetrics}
            />
          ) : null}
        </CardContent>
      </Card>
    </div>
  )
}
