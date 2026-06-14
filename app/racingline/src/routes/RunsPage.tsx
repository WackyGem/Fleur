import { useMemo, useState } from "react"
import { Link } from "react-router-dom"
import { Add01Icon, ArrowReloadHorizontalIcon } from "@hugeicons/core-free-icons"

import { useRuleSetsQuery, useRunsQuery } from "@/api/hooks"
import { MissingBackendState, TableSkeleton } from "@/components/racingline/data-state"
import { FilterSelect } from "@/components/racingline/filter-select"
import { RacinglineIcon } from "@/components/racingline/icon"
import { RunsTable } from "@/features/runs/components/runs-table"
import { isFailureStatus, isRunActiveStatus } from "@/lib/status"
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
  Field,
  FieldGroup,
  FieldLabel,
} from "@/components/ui/field"
import { Input } from "@/components/ui/input"
import type { RunRecord, RunsQuery } from "@/types/rearview"

const EMPTY_RUNS: RunRecord[] = []

const statusOptions = [
  { label: "All status", value: null },
  { label: "created", value: "created" },
  { label: "validating", value: "validating" },
  { label: "compiling", value: "compiling" },
  { label: "running_clickhouse", value: "running_clickhouse" },
  { label: "writing_pool", value: "writing_pool" },
  { label: "writing_signals", value: "writing_signals" },
  { label: "succeeded", value: "succeeded" },
  { label: "failed", value: "failed" },
  { label: "cancelled", value: "cancelled" },
]

export function RunsPage() {
  const [filters, setFilters] = useState<RunsQuery>({
    limit: 50,
    offset: 0,
  })

  const runsQuery = useRunsQuery(filters)
  const ruleSetsQuery = useRuleSetsQuery({ limit: 100, offset: 0 })
  const runs = runsQuery.data?.items ?? EMPTY_RUNS

  const ruleSetOptions = useMemo(
    () => [
      { label: "All rule sets", value: null },
      ...(ruleSetsQuery.data?.items ?? []).map((ruleSet) => ({
        label: ruleSet.name,
        value: ruleSet.rule_set_id,
      })),
    ],
    [ruleSetsQuery.data?.items],
  )

  const summary = useMemo(
    () => ({
      active: runs.filter((run) => isRunActiveStatus(run.status)).length,
      succeeded: runs.filter((run) => run.status === "succeeded").length,
      failed: runs.filter((run) => isFailureStatus(run.status)).length,
      total: runs.length,
    }),
    [runs],
  )

  function patchFilters(patch: Partial<RunsQuery>) {
    setFilters((current) => ({ ...current, ...patch, offset: 0 }))
  }

  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
        <div>
          <h1 className="text-xl font-medium">Runs</h1>
          <p className="text-sm text-muted-foreground">
            Recent Rearview screening runs and execution state.
          </p>
        </div>
        <div className="flex gap-2">
          <Button
            disabled={runsQuery.isFetching}
            onClick={() => void runsQuery.refetch()}
            size="sm"
            variant="outline"
          >
            <RacinglineIcon icon={ArrowReloadHorizontalIcon} inline="start" />
            Refresh
          </Button>
          <Button
            nativeButton={false}
            render={<Link to="/rules" />}
            size="sm"
          >
            <RacinglineIcon icon={Add01Icon} inline="start" />
            New rule
          </Button>
        </div>
      </div>

      <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
        <SummaryCard label="Total" value={summary.total} />
        <SummaryCard label="Active" value={summary.active} />
        <SummaryCard label="Succeeded" value={summary.succeeded} />
        <SummaryCard label="Failed" value={summary.failed} />
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Run filters</CardTitle>
          <CardDescription>Server-side filters sent to Rearview.</CardDescription>
          <CardAction>
            <Button
              onClick={() => setFilters({ limit: 50, offset: 0 })}
              size="sm"
              variant="ghost"
            >
              Reset
            </Button>
          </CardAction>
        </CardHeader>
        <CardContent>
          <FieldGroup className="grid gap-3 sm:grid-cols-2 lg:grid-cols-5">
            <Field>
              <FieldLabel>Status</FieldLabel>
              <FilterSelect
                className="w-full"
                onValueChange={(status) =>
                  patchFilters({
                    status: status === "failed" ? undefined : status,
                    keyword:
                      status === "failed" ? "failed" : filters.keyword ?? "",
                  })
                }
                options={statusOptions}
                value={
                  filters.keyword === "failed" && !filters.status
                    ? "failed"
                    : filters.status ?? ""
                }
              />
            </Field>
            <Field>
              <FieldLabel>Rule set</FieldLabel>
              <FilterSelect
                className="w-full"
                onValueChange={(rule_set_id) =>
                  patchFilters({ rule_set_id: rule_set_id || undefined })
                }
                options={ruleSetOptions}
                value={filters.rule_set_id ?? ""}
              />
            </Field>
            <Field>
              <FieldLabel>Start date</FieldLabel>
              <Input
                onChange={(event) =>
                  patchFilters({ start_date: event.currentTarget.value })
                }
                type="date"
                value={filters.start_date ?? ""}
              />
            </Field>
            <Field>
              <FieldLabel>End date</FieldLabel>
              <Input
                onChange={(event) =>
                  patchFilters({ end_date: event.currentTarget.value })
                }
                type="date"
                value={filters.end_date ?? ""}
              />
            </Field>
            <Field>
              <FieldLabel>Keyword</FieldLabel>
              <Input
                onChange={(event) =>
                  patchFilters({ keyword: event.currentTarget.value })
                }
                placeholder="run, version, rule set"
                value={filters.keyword ?? ""}
              />
            </Field>
          </FieldGroup>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Run list</CardTitle>
          <CardDescription>
            {runsQuery.data?.total === undefined
              ? `${runs.length} loaded`
              : `${runs.length} of ${runsQuery.data.total} loaded`}
          </CardDescription>
        </CardHeader>
        <CardContent>
          {runsQuery.isPending ? <TableSkeleton /> : null}
          {runsQuery.isError ? (
            <MissingBackendState
              description="GET /rearview/runs did not return a usable list response."
              retry={() => void runsQuery.refetch()}
              title="Run list API unavailable"
            />
          ) : null}
          {runsQuery.isSuccess && runs.length === 0 ? (
            <MissingBackendState
              description="Rearview returned a real empty run list for the current filters."
              title="No runs"
            />
          ) : null}
          {runs.length > 0 ? <RunsTable runs={runs} /> : null}
        </CardContent>
      </Card>
    </div>
  )
}

function SummaryCard({ label, value }: { label: string; value: number }) {
  return (
    <Card size="sm">
      <CardHeader>
        <CardTitle>{label}</CardTitle>
        <CardDescription>{value.toLocaleString()}</CardDescription>
      </CardHeader>
    </Card>
  )
}
