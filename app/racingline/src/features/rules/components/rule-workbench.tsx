import { useMemo, useState } from "react"
import { useNavigate } from "react-router-dom"
import {
  Add01Icon,
  ArrowRight01Icon,
  CheckmarkCircle02Icon,
  PlayIcon,
} from "@hugeicons/core-free-icons"

import {
  useCreateRuleSetMutation,
  useCreateRuleVersionMutation,
  useCreateRunMutation,
  useExplainMutation,
  useMetricsQuery,
  useRuleSetsQuery,
  useRuleVersionsQuery,
} from "@/api/hooks"
import { ErrorState, MissingBackendState, TableSkeleton } from "@/components/racingline/data-state"
import { FilterSelect } from "@/components/racingline/filter-select"
import { RacinglineIcon } from "@/components/racingline/icon"
import { StatusBadge } from "@/components/racingline/status-badge"
import { AccountTemplateCard } from "@/features/portfolio/components/account-template-card"
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
import { Checkbox } from "@/components/ui/checkbox"
import {
  Field,
  FieldContent,
  FieldDescription,
  FieldGroup,
  FieldLabel,
  FieldLegend,
  FieldSet,
} from "@/components/ui/field"
import { Input } from "@/components/ui/input"
import { Textarea } from "@/components/ui/textarea"
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table"
import {
  buildLowReversalRuleVersionSpec,
  buildRuleVersionSpec,
  splitCsv,
  useWorkbenchStore,
} from "@/store/workbench"
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group"
import type { MetricDefinition, Operator, RuleVersionRecord } from "@/types/rearview"

const operatorOptions: Array<{ label: string; value: Operator }> = [
  { label: "eq", value: "eq" },
  { label: "ne", value: "ne" },
  { label: "lt", value: "lt" },
  { label: "lte", value: "lte" },
  { label: "gt", value: "gt" },
  { label: "gte", value: "gte" },
  { label: "between", value: "between" },
  { label: "is_null", value: "is_null" },
]

export function RuleWorkbench() {
  const navigate = useNavigate()
  const draft = useWorkbenchStore((state) => state.draft)
  const setDraft = useWorkbenchStore((state) => state.setDraft)
  const [manualRuleSetId, setManualRuleSetId] = useState("")
  const [manualVersionId, setManualVersionId] = useState("")
  const [ruleMode, setRuleMode] = useState<"preset" | "simple">("preset")

  const ruleSetsQuery = useRuleSetsQuery({ limit: 100, offset: 0 })
  const selectedRuleSetId =
    manualRuleSetId || ruleSetsQuery.data?.items[0]?.rule_set_id || ""
  const versionsQuery = useRuleVersionsQuery(selectedRuleSetId, {
    limit: 100,
    offset: 0,
  })
  const versionItems = versionsQuery.data?.items ?? []
  const selectedVersionId =
    manualVersionId && versionItems.some((version) => version.rule_version_id === manualVersionId)
      ? manualVersionId
      : versionItems[0]?.rule_version_id || ""
  const metricsQuery = useMetricsQuery()
  const explainMutation = useExplainMutation()
  const createRuleSetMutation = useCreateRuleSetMutation()
  const createRuleVersionMutation = useCreateRuleVersionMutation(selectedRuleSetId)
  const createRunMutation = useCreateRunMutation()

  const metrics = metricsQuery.data ?? []
  const filterMetrics = metrics.filter((metric) => metric.allow_filter)
  const scoringMetrics = metrics.filter((metric) => metric.allow_scoring)
  const outputMetrics = metrics.filter((metric) => metric.default_output)

  const metricOptions = useMemo(
    () => [
      { label: "Select metric", value: null },
      ...filterMetrics.map((metric) => ({
        label: metric.logical_metric,
        value: metric.logical_metric,
      })),
    ],
    [filterMetrics],
  )
  const scoringOptions = useMemo(
    () => [
      { label: "Use filter metric", value: null },
      ...scoringMetrics.map((metric) => ({
        label: metric.logical_metric,
        value: metric.logical_metric,
      })),
    ],
    [scoringMetrics],
  )
  const ruleSpec = useMemo(
    () =>
      ruleMode === "preset"
        ? buildLowReversalRuleVersionSpec(draft)
        : buildRuleVersionSpec(draft),
    [draft, ruleMode],
  )
  const selectedVersion = versionItems.find(
    (version) => version.rule_version_id === selectedVersionId,
  )

  async function explain() {
    await explainMutation.mutateAsync({
      rule: ruleSpec,
      range:
        draft.runStartDate && draft.runEndDate
          ? {
              end_date: draft.runEndDate,
              start_date: draft.runStartDate,
              top_n: Number(draft.runTopN || draft.topNDefault || 20),
            }
          : undefined,
    })
  }

  async function publishVersion() {
    const ruleSetId =
      selectedRuleSetId ||
      (
        await createRuleSetMutation.mutateAsync({
          description: draft.ruleSetDescription || undefined,
          name: draft.ruleSetName || `racingline-${Date.now()}`,
          owner: draft.owner || "racingline",
        })
      ).rule_set_id

    setManualRuleSetId(ruleSetId)
    const version = await createRuleVersionMutation.mutateAsync({
      request: {
        activate: true,
        created_by: draft.owner || "racingline",
        rule: ruleSpec,
      },
      targetRuleSetId: ruleSetId,
    })
    setManualVersionId(version.rule_version_id)
  }

  async function startRun() {
    if (!selectedVersionId || !draft.runStartDate || !draft.runEndDate) {
      return
    }

    const run = await createRunMutation.mutateAsync({
      end_date: draft.runEndDate,
      rule_version_id: selectedVersionId,
      start_date: draft.runStartDate,
      top_n: Number(draft.runTopN || selectedVersion?.top_n_default || 20),
    })
    navigate(`/runs/${run.run_id}`)
  }

  return (
    <div className="grid gap-4 xl:grid-cols-[22rem_1fr]">
      <div className="flex flex-col gap-4">
        <Card>
          <CardHeader>
            <CardTitle>Rule sets</CardTitle>
            <CardDescription>
              {ruleSetsQuery.data?.items.length ?? 0} loaded
            </CardDescription>
          </CardHeader>
          <CardContent>
            {ruleSetsQuery.isPending ? <TableSkeleton rows={3} /> : null}
            {ruleSetsQuery.isError ? (
              <MissingBackendState
                description="GET /rearview/rule-sets did not return a usable list response."
                retry={() => void ruleSetsQuery.refetch()}
                title="Rule set list API unavailable"
              />
            ) : null}
            <div className="flex flex-col gap-2">
              {(ruleSetsQuery.data?.items ?? []).map((ruleSet) => (
                <Button
                  key={ruleSet.rule_set_id}
                  onClick={() => {
                    setManualRuleSetId(ruleSet.rule_set_id)
                    setManualVersionId(ruleSet.current_version_id ?? "")
                  }}
                  size="sm"
                  variant={
                    selectedRuleSetId === ruleSet.rule_set_id
                      ? "secondary"
                      : "ghost"
                  }
                >
                  <span className="truncate">{ruleSet.name}</span>
                </Button>
              ))}
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>Versions</CardTitle>
            <CardDescription>
              {versionsQuery.data?.items.length ?? 0} loaded
            </CardDescription>
          </CardHeader>
          <CardContent>
            {versionsQuery.isPending && selectedRuleSetId ? (
              <TableSkeleton rows={3} />
            ) : null}
            {versionsQuery.isError ? (
              <MissingBackendState
                description="GET /rearview/rule-sets/{rule_set_id}/versions did not return a usable list response."
                retry={() => void versionsQuery.refetch()}
                title="Rule versions API unavailable"
              />
            ) : null}
            <div className="flex flex-col gap-2">
              {versionItems.map((version) => (
                <VersionButton
                  key={version.rule_version_id}
                  active={version.rule_version_id === selectedVersionId}
                  onClick={() => setManualVersionId(version.rule_version_id)}
                  version={version}
                />
              ))}
            </div>
          </CardContent>
        </Card>
      </div>

      <div className="flex min-w-0 flex-col gap-4">
        <Card>
          <CardHeader>
            <CardTitle>Rule draft</CardTitle>
            <CardDescription>
              {ruleMode === "preset"
                ? "Low reversal preset"
                : `${metrics.length} catalog metrics available`}
            </CardDescription>
            <CardAction>
              <ToggleGroup
                onValueChange={(nextValue) => {
                  const nextMode = nextValue[0]
                  if (nextMode === "preset" || nextMode === "simple") {
                    setRuleMode(nextMode)
                  }
                }}
                size="sm"
                spacing={0}
                value={[ruleMode]}
                variant="outline"
              >
                <ToggleGroupItem aria-label="Preset rule" value="preset">
                  Preset
                </ToggleGroupItem>
                <ToggleGroupItem aria-label="Simple rule" value="simple">
                  Simple
                </ToggleGroupItem>
              </ToggleGroup>
            </CardAction>
          </CardHeader>
          <CardContent>
            {metricsQuery.isError ? (
              <ErrorState
                error={metricsQuery.error}
                title="Metric catalog API returned an error"
              />
            ) : null}
            <div className="grid gap-5 lg:grid-cols-2">
              <FieldSet>
                <FieldLegend>Rule set</FieldLegend>
                <FieldGroup>
                  <Field>
                    <FieldLabel>Name</FieldLabel>
                    <Input
                      onChange={(event) =>
                        setDraft({ ruleSetName: event.currentTarget.value })
                      }
                      value={draft.ruleSetName}
                    />
                  </Field>
                  <Field>
                    <FieldLabel>Description</FieldLabel>
                    <Textarea
                      onChange={(event) =>
                        setDraft({
                          ruleSetDescription: event.currentTarget.value,
                        })
                      }
                      value={draft.ruleSetDescription}
                    />
                  </Field>
                  <Field>
                    <FieldLabel>Owner</FieldLabel>
                    <Input
                      onChange={(event) =>
                        setDraft({ owner: event.currentTarget.value })
                      }
                      value={draft.owner}
                    />
                  </Field>
                </FieldGroup>
              </FieldSet>

              <FieldSet>
                <FieldLegend>Universe</FieldLegend>
                <FieldGroup>
                  <Field>
                    <FieldLabel>Base</FieldLabel>
                    <Input
                      onChange={(event) =>
                        setDraft({ universeBase: event.currentTarget.value })
                      }
                      value={draft.universeBase}
                    />
                  </Field>
                  <Field orientation="horizontal">
                    <Checkbox
                      checked={draft.excludeSt}
                      id="exclude-st"
                      onCheckedChange={(checked) =>
                        setDraft({ excludeSt: checked })
                      }
                    />
                    <FieldContent>
                      <FieldLabel htmlFor="exclude-st">Exclude ST</FieldLabel>
                    </FieldContent>
                  </Field>
                  <Field orientation="horizontal">
                    <Checkbox
                      checked={draft.excludeSuspend}
                      id="exclude-suspend"
                      onCheckedChange={(checked) =>
                        setDraft({ excludeSuspend: checked })
                      }
                    />
                    <FieldContent>
                      <FieldLabel htmlFor="exclude-suspend">
                        Exclude suspend
                      </FieldLabel>
                    </FieldContent>
                  </Field>
                  <Field>
                    <FieldLabel>Include security codes</FieldLabel>
                    <Input
                      onChange={(event) =>
                        setDraft({
                          includeSecurityCodes: event.currentTarget.value,
                        })
                      }
                      value={draft.includeSecurityCodes}
                    />
                  </Field>
                  <Field>
                    <FieldLabel>Exclude security codes</FieldLabel>
                    <Input
                      onChange={(event) =>
                        setDraft({
                          excludeSecurityCodes: event.currentTarget.value,
                        })
                      }
                      value={draft.excludeSecurityCodes}
                    />
                  </Field>
                </FieldGroup>
              </FieldSet>

              <FieldSet>
                <FieldLegend>Pool filter</FieldLegend>
                <FieldGroup>
                  <Field>
                    <FieldLabel>Metric</FieldLabel>
                    <FilterSelect
                      className="w-full"
                      onValueChange={(filterMetric) =>
                        setDraft({ filterMetric })
                      }
                      options={metricOptions}
                      value={draft.filterMetric}
                    />
                  </Field>
                  <Field>
                    <FieldLabel>Operator</FieldLabel>
                    <FilterSelect
                      className="w-full"
                      onValueChange={(filterOperator) =>
                        setDraft({ filterOperator: filterOperator as Operator })
                      }
                      options={operatorOptions}
                      value={draft.filterOperator}
                    />
                  </Field>
                  <Field>
                    <FieldLabel>Operand</FieldLabel>
                    <Input
                      disabled={draft.filterOperator === "is_null"}
                      onChange={(event) =>
                        setDraft({ filterValue: event.currentTarget.value })
                      }
                      type="number"
                      value={draft.filterValue}
                    />
                  </Field>
                </FieldGroup>
              </FieldSet>

              <FieldSet>
                <FieldLegend>Scoring</FieldLegend>
                <FieldGroup>
                  <Field>
                    <FieldLabel>Weighted metric</FieldLabel>
                    <FilterSelect
                      className="w-full"
                      onValueChange={(scoringMetric) =>
                        setDraft({ scoringMetric })
                      }
                      options={scoringOptions}
                      value={draft.scoringMetric}
                    />
                  </Field>
                  <Field>
                    <FieldLabel>Weight</FieldLabel>
                    <Input
                      onChange={(event) =>
                        setDraft({ scoringWeight: event.currentTarget.value })
                      }
                      type="number"
                      value={draft.scoringWeight}
                    />
                  </Field>
                  <div className="grid gap-3 sm:grid-cols-2">
                    <Field>
                      <FieldLabel>Clamp min</FieldLabel>
                      <Input
                        onChange={(event) =>
                          setDraft({ clampMin: event.currentTarget.value })
                        }
                        type="number"
                        value={draft.clampMin}
                      />
                    </Field>
                    <Field>
                      <FieldLabel>Clamp max</FieldLabel>
                      <Input
                        onChange={(event) =>
                          setDraft({ clampMax: event.currentTarget.value })
                        }
                        type="number"
                        value={draft.clampMax}
                      />
                    </Field>
                  </div>
                </FieldGroup>
              </FieldSet>

              <FieldSet>
                <FieldLegend>Output metrics</FieldLegend>
                <FieldDescription>
                  {splitCsv(draft.outputMetrics).length} selected
                </FieldDescription>
                <FieldGroup className="max-h-72 overflow-auto">
                  {outputMetrics.map((metric) => (
                    <MetricCheckbox
                      key={metric.logical_metric}
                      draftValue={draft.outputMetrics}
                      metric={metric}
                      onChange={(outputMetricsValue) =>
                        setDraft({ outputMetrics: outputMetricsValue })
                      }
                    />
                  ))}
                </FieldGroup>
              </FieldSet>

              <FieldSet>
                <FieldLegend>Run request</FieldLegend>
                <FieldGroup>
                  <Field>
                    <FieldLabel>TopN default</FieldLabel>
                    <Input
                      onChange={(event) =>
                        setDraft({ topNDefault: event.currentTarget.value })
                      }
                      type="number"
                      value={draft.topNDefault}
                    />
                  </Field>
                  <div className="grid gap-3 sm:grid-cols-2">
                    <Field>
                      <FieldLabel>Start date</FieldLabel>
                      <Input
                        onChange={(event) =>
                          setDraft({ runStartDate: event.currentTarget.value })
                        }
                        type="date"
                        value={draft.runStartDate}
                      />
                    </Field>
                    <Field>
                      <FieldLabel>End date</FieldLabel>
                      <Input
                        onChange={(event) =>
                          setDraft({ runEndDate: event.currentTarget.value })
                        }
                        type="date"
                        value={draft.runEndDate}
                      />
                    </Field>
                  </div>
                  <Field>
                    <FieldLabel>Run top_n</FieldLabel>
                    <Input
                      onChange={(event) =>
                        setDraft({ runTopN: event.currentTarget.value })
                      }
                      type="number"
                      value={draft.runTopN}
                    />
                  </Field>
                </FieldGroup>
              </FieldSet>
            </div>
          </CardContent>
        </Card>

        {selectedRuleSetId ? (
          <AccountTemplateCard
            ruleSetId={selectedRuleSetId}
            topN={Number(draft.topNDefault || draft.runTopN || 20)}
          />
        ) : null}

        <Card>
          <CardHeader>
            <CardTitle>Explain and publish</CardTitle>
            <CardDescription>
              {explainMutation.data?.sql_hash ??
                explainMutation.data?.compiled_sql_hash ??
                "No successful explain yet"}
            </CardDescription>
            <CardAction>
              <div className="flex gap-2">
                <Button
                  disabled={
                    explainMutation.isPending ||
                    (ruleMode === "simple" && !draft.filterMetric)
                  }
                  onClick={() => void explain()}
                  size="sm"
                  variant="outline"
                >
                  <RacinglineIcon
                    icon={CheckmarkCircle02Icon}
                    inline="start"
                  />
                  Explain
                </Button>
                <Button
                  disabled={
                    !explainMutation.data ||
                    createRuleSetMutation.isPending ||
                    createRuleVersionMutation.isPending
                  }
                  onClick={() => void publishVersion()}
                  size="sm"
                >
                  <RacinglineIcon icon={Add01Icon} inline="start" />
                  Publish
                </Button>
                <Button
                  disabled={
                    !selectedVersionId ||
                    !draft.runStartDate ||
                    !draft.runEndDate ||
                    createRunMutation.isPending
                  }
                  onClick={() => void startRun()}
                  size="sm"
                  variant="secondary"
                >
                  <RacinglineIcon icon={PlayIcon} inline="start" />
                  Run
                  <RacinglineIcon icon={ArrowRight01Icon} inline="end" />
                </Button>
              </div>
            </CardAction>
          </CardHeader>
          <CardContent>
            <div className="grid gap-3 lg:grid-cols-2">
              <div className="flex flex-col gap-3">
                {explainMutation.isError ? (
                  <ErrorState
                    error={explainMutation.error}
                    title="Explain failed"
                  />
                ) : null}
                {createRuleSetMutation.isError ? (
                  <ErrorState
                    error={createRuleSetMutation.error}
                    title="Rule set creation failed"
                  />
                ) : null}
                {createRuleVersionMutation.isError ? (
                  <ErrorState
                    error={createRuleVersionMutation.error}
                    title="Rule version publish failed"
                  />
                ) : null}
                {createRunMutation.isError ? (
                  <ErrorState
                    error={createRunMutation.error}
                    title="Run creation failed"
                  />
                ) : null}
                {explainMutation.data ? (
                  <ExplainSummary
                    chunkCount={explainMutation.data.chunk_plan?.length ?? 0}
                    columns={formatRequiredColumns(
                      explainMutation.data.required_columns,
                    )}
                    marts={explainMutation.data.required_marts ?? []}
                    metrics={explainMutation.data.required_metrics ?? []}
                  />
                ) : (
                  <MissingBackendState
                    description="A successful explain response is required before publish."
                    title="No explain result"
                  />
                )}
              </div>
              <pre className="max-h-96 overflow-auto rounded-lg bg-muted p-3 text-xs">
                {JSON.stringify(ruleSpec, null, 2)}
              </pre>
            </div>
          </CardContent>
        </Card>
      </div>
    </div>
  )
}

function VersionButton({
  active,
  onClick,
  version,
}: {
  active: boolean
  onClick: () => void
  version: RuleVersionRecord
}) {
  return (
    <Button
      className="h-auto justify-start"
      onClick={onClick}
      variant={active ? "secondary" : "ghost"}
    >
      <div className="flex min-w-0 flex-col items-start gap-1">
        <span className="text-sm">v{version.version_no}</span>
        <span className="truncate font-mono text-xs">
          {version.rule_version_id}
        </span>
        <StatusBadge status={version.status} />
      </div>
    </Button>
  )
}

function MetricCheckbox({
  draftValue,
  metric,
  onChange,
}: {
  draftValue: string
  metric: MetricDefinition
  onChange: (value: string) => void
}) {
  const current = splitCsv(draftValue)
  const checked = current.includes(metric.logical_metric)

  return (
    <Field orientation="horizontal">
      <Checkbox
        checked={checked}
        id={`output-${metric.logical_metric}`}
        onCheckedChange={(nextChecked) => {
          const nextValues = nextChecked
            ? [...current, metric.logical_metric]
            : current.filter((value) => value !== metric.logical_metric)
          onChange(nextValues.join(", "))
        }}
      />
      <FieldContent>
        <FieldLabel htmlFor={`output-${metric.logical_metric}`}>
          {metric.logical_metric}
        </FieldLabel>
        <FieldDescription>
          {metric.mart_table}.{metric.column_name}
        </FieldDescription>
      </FieldContent>
    </Field>
  )
}

function ExplainSummary({
  chunkCount,
  columns,
  marts,
  metrics,
}: {
  chunkCount: number
  columns: string[]
  marts: string[]
  metrics: string[]
}) {
  return (
    <Table>
      <TableHeader>
        <TableRow>
          <TableHead>section</TableHead>
          <TableHead>values</TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        <SummaryRow label="required_metrics" values={metrics} />
        <SummaryRow label="required_marts" values={marts} />
        <SummaryRow label="required_columns" values={columns} />
        <TableRow>
          <TableCell>chunk_plan</TableCell>
          <TableCell>
            <Badge variant="secondary">{chunkCount}</Badge>
          </TableCell>
        </TableRow>
      </TableBody>
    </Table>
  )
}

function SummaryRow({ label, values }: { label: string; values: string[] }) {
  return (
    <TableRow>
      <TableCell>{label}</TableCell>
      <TableCell>
        <div className="flex flex-wrap gap-1">
          {values.length === 0 ? (
            <Badge variant="outline">-</Badge>
          ) : (
            values.map((value) => (
              <Badge key={value} variant="secondary">
                {value}
              </Badge>
            ))
          )}
        </div>
      </TableCell>
    </TableRow>
  )
}

function formatRequiredColumns(columns?: Record<string, string[]>) {
  if (!columns) {
    return []
  }

  return Object.entries(columns).map(
    ([mart, names]) => `${mart}: ${names.join(", ")}`,
  )
}
