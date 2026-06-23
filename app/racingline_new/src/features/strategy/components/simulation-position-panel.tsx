import type { ReactNode } from "react"

import {
  Alert,
  AlertAction,
  AlertDescription,
  AlertTitle,
} from "@/components/ui/alert"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import {
  Card,
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
  FieldTitle,
} from "@/components/ui/field"
import {
  InputGroup,
  InputGroupAddon,
  InputGroupInput,
} from "@/components/ui/input-group"
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectLabel,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import { Separator } from "@/components/ui/separator"
import { Slider } from "@/components/ui/slider"
import { Skeleton } from "@/components/ui/skeleton"
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table"
import { StrategySplitPanel } from "@/features/strategy/components/strategy-split-panel"
import {
  buildPoolCountTrendData,
  type PoolCountTrendPoint,
} from "@/features/strategy/pool-count-trend"
import type { PreviewSnapshot } from "@/features/strategy/preview"
import type {
  IndicatorCatalog,
  SimulationSettings,
} from "@/features/strategy/types"
import {
  getCatalog,
  getCatalogMetricsByType,
  getTrendMovingAverageCatalogs,
} from "@/features/strategy/utils"
import { cn } from "@/lib/utils"

type SimulationPositionPanelProps = {
  backtestValidationError: string | null
  catalogOptions: IndicatorCatalog[]
  commissionRateMaxPercent: number | null
  isBacktestValidationPending: boolean
  isMarketTemplateError: boolean
  isMarketTemplateLoading: boolean
  marketTemplateError: unknown
  onRetryMarketTemplate: () => void
  onSettingsChange: (settings: SimulationSettings) => void
  previewSnapshot: PreviewSnapshot | null
  settings: SimulationSettings
}

type NumberInputFieldProps = {
  ariaInvalid?: boolean
  disabled?: boolean
  invalid?: boolean
  label: string
  max?: number
  min?: number
  onValueChange: (value: number) => void
  step?: number
  suffix?: string
  value: number
}

type ReadonlySelectFieldProps = {
  label: string
  value: string
}

type RiskRuleRowProps = {
  checked: boolean
  children: ReactNode
  disabled?: boolean
  onCheckedChange: (checked: boolean) => void
  title: string
}

type SettingRowProps = {
  children: ReactNode
  label: ReactNode
}

type SummaryRow = {
  condition: string
  trigger: string
}

type RiskSettingsKey =
  | "fixedStopLoss"
  | "indicatorStopLoss"
  | "takeProfit"
  | "timeStopLoss"

type TransactionFeeSettingsKey = keyof SimulationSettings["transactionFees"]

type TransactionFeeRow = {
  direction: string
  key: TransactionFeeSettingsKey
  name: string
  note?: string
  step: number
}

const sectionCardClassName = "bg-transparent ring-0"
const configSectionCardClassName = cn(sectionCardClassName, "xl:pr-4")
const configSeparatorClassName = "bg-border/60 md:ml-[11rem] md:max-w-[37rem]"
const configListClassName = "max-w-[48rem] gap-0"
const sectionSeparatorClassName = "max-w-[52rem] bg-border/60"
const settingRowClassName =
  "grid gap-3 py-2 md:grid-cols-[11rem_minmax(0,1fr)] md:items-center"
const settingControlGridClassName =
  "grid gap-2 md:grid-cols-[6.5rem_10rem_minmax(0,1fr)] md:items-center"
const transactionFeeRowClassName =
  "grid gap-2 py-2 md:grid-cols-[11rem_6.5rem_10rem_minmax(0,1fr)] md:items-center"
const buyRuleLabel = "T+1日开盘价买入"
const rebalanceRuleLabel = "仓位空余按信号调入"

const transactionFeeRows: TransactionFeeRow[] = [
  {
    direction: "卖出",
    key: "stampDutyRatePercent",
    name: "印花税",
    step: 0.001,
  },
  {
    direction: "双向",
    key: "transferFeeRatePercent",
    name: "过户费",
    step: 0.001,
  },
  {
    direction: "双向",
    key: "commissionRatePercent",
    name: "佣金",
    step: 0.001,
  },
  {
    direction: "双向",
    key: "slippageRatePercent",
    name: "成交滑点",
    note: "买入上浮，卖出下浮",
    step: 0.001,
  },
]

function SimulationPositionPanel({
  backtestValidationError,
  catalogOptions,
  commissionRateMaxPercent,
  isBacktestValidationPending,
  isMarketTemplateError,
  isMarketTemplateLoading,
  marketTemplateError,
  onRetryMarketTemplate,
  onSettingsChange,
  previewSnapshot,
  settings,
}: SimulationPositionPanelProps) {
  const activeRiskRows = buildRiskSummaryRows(settings)
  const maxPositions = Math.max(1, Math.floor(settings.maxPositions))
  const targetWeight = Math.min(
    1 / maxPositions,
    settings.singlePositionLimitPercent / 100
  )
  const perPositionCapital = settings.initialCapital * Math.max(0, targetWeight)
  const poolCountTrend = buildPoolCountTrendData(previewSnapshot)
  const latestPoolCount = poolCountTrend.at(-1)?.count ?? 0

  function updateSettings(patch: Partial<SimulationSettings>) {
    onSettingsChange({ ...settings, ...patch })
  }

  function updateRiskSettings<Key extends RiskSettingsKey>(
    key: Key,
    patch: Partial<SimulationSettings[Key]>
  ) {
    onSettingsChange({
      ...settings,
      [key]: {
        ...(settings[key] as object),
        ...patch,
      },
    })
  }

  function updateTransactionFeeSettings(
    key: TransactionFeeSettingsKey,
    value: number
  ) {
    onSettingsChange({
      ...settings,
      transactionFees: {
        ...settings.transactionFees,
        [key]: value,
      },
    })
  }

  return (
    <StrategySplitPanel
      mainClassName="gap-3"
      main={
        <>
          <Card size="sm" className={cn(configSectionCardClassName, "pt-0")}>
            <CardHeader className="px-0">
              <CardTitle>仓位管理</CardTitle>
            </CardHeader>
            <CardContent className="px-0">
              <FieldSet>
                <FieldLegend className="sr-only">仓位管理</FieldLegend>
                <FieldGroup className="grid max-w-[54rem] gap-3 md:grid-cols-3">
                  <NumberInputField
                    label="初始金额"
                    min={0}
                    onValueChange={(initialCapital) =>
                      updateSettings({ initialCapital })
                    }
                    step={10000}
                    suffix="元"
                    value={settings.initialCapital}
                  />

                  <NumberInputField
                    label="最大持仓"
                    min={1}
                    onValueChange={(maxPositions) =>
                      updateSettings({
                        maxPositions: Math.max(1, Math.floor(maxPositions)),
                      })
                    }
                    step={1}
                    suffix="只"
                    value={settings.maxPositions}
                  />

                  <Field>
                    <FieldLabel>单票上限</FieldLabel>
                    <div className="grid grid-cols-[minmax(0,1fr)_7rem] items-center gap-3">
                      <Slider
                        aria-label="单票仓位上限"
                        max={100}
                        min={1}
                        onValueChange={(nextValue) =>
                          updateSettings({
                            singlePositionLimitPercent: readSliderValue(
                              nextValue,
                              settings.singlePositionLimitPercent
                            ),
                          })
                        }
                        step={1}
                        value={[settings.singlePositionLimitPercent]}
                      />
                      <InputGroup>
                        <InputGroupInput
                          inputMode="decimal"
                          max={100}
                          min={1}
                          onChange={(event) =>
                            updateSettings({
                              singlePositionLimitPercent: toBoundedNumber(
                                event.target.value,
                                1,
                                100
                              ),
                            })
                          }
                          step={1}
                          type="number"
                          value={String(settings.singlePositionLimitPercent)}
                        />
                        <InputGroupAddon align="inline-end">%</InputGroupAddon>
                      </InputGroup>
                    </div>
                  </Field>

                  <NumberInputField
                    label="买入信号 Top N"
                    min={1}
                    onValueChange={(buyTopN) =>
                      updateSettings({
                        buyTopN: Math.max(1, Math.floor(buyTopN)),
                      })
                    }
                    step={1}
                    suffix="只"
                    value={settings.buyTopN}
                  />

                  <ReadonlySelectField label="买入规则" value={buyRuleLabel} />

                  <ReadonlySelectField
                    label="调仓规则"
                    value={rebalanceRuleLabel}
                  />
                </FieldGroup>
              </FieldSet>
            </CardContent>
          </Card>

          <Separator className={sectionSeparatorClassName} />

          <Card size="sm" className={configSectionCardClassName}>
            <CardHeader className="px-0">
              <CardTitle>交易费率</CardTitle>
            </CardHeader>
            <CardContent className="px-0">
              <TransactionFeeList
                commissionRateMaxPercent={commissionRateMaxPercent}
                fees={settings.transactionFees}
                isTemplateError={isMarketTemplateError}
                isTemplateLoading={isMarketTemplateLoading}
                onRetryTemplate={onRetryMarketTemplate}
                onRateChange={updateTransactionFeeSettings}
                templateError={marketTemplateError}
              />
            </CardContent>
          </Card>

          <Separator className={sectionSeparatorClassName} />

          <Card size="sm" className={configSectionCardClassName}>
            <CardHeader className="px-0">
              <CardTitle>风险管理</CardTitle>
            </CardHeader>
            <CardContent className="px-0">
              <FieldSet>
                <FieldLegend className="sr-only">卖出条件</FieldLegend>
                <FieldGroup className={configListClassName}>
                  <RiskRuleRow
                    checked={settings.takeProfit.enabled}
                    onCheckedChange={(enabled) =>
                      updateRiskSettings("takeProfit", { enabled })
                    }
                    title="固定止盈"
                  >
                    <div className={settingControlGridClassName}>
                      <div className="text-xs text-muted-foreground">
                        收益达到
                      </div>
                      <CompactNumberInput
                        disabled={!settings.takeProfit.enabled}
                        label="收益达到"
                        min={0}
                        onValueChange={(profitPercent) =>
                          updateRiskSettings("takeProfit", { profitPercent })
                        }
                        step={0.5}
                        suffix="%"
                        value={settings.takeProfit.profitPercent}
                      />
                    </div>
                  </RiskRuleRow>
                  <Separator className={configSeparatorClassName} />

                  <RiskRuleRow
                    checked={settings.fixedStopLoss.enabled}
                    onCheckedChange={(enabled) =>
                      updateRiskSettings("fixedStopLoss", { enabled })
                    }
                    title="固定止损"
                  >
                    <div className={settingControlGridClassName}>
                      <div className="text-xs text-muted-foreground">
                        买入价下跌
                      </div>
                      <CompactNumberInput
                        disabled={!settings.fixedStopLoss.enabled}
                        label="买入价下跌"
                        min={0}
                        onValueChange={(lossPercent) =>
                          updateRiskSettings("fixedStopLoss", { lossPercent })
                        }
                        step={0.5}
                        suffix="%"
                        value={settings.fixedStopLoss.lossPercent}
                      />
                    </div>
                  </RiskRuleRow>
                  <Separator className={configSeparatorClassName} />

                  <RiskRuleRow
                    checked={settings.indicatorStopLoss.enabled}
                    onCheckedChange={(enabled) =>
                      updateRiskSettings("indicatorStopLoss", { enabled })
                    }
                    title="指标止损"
                  >
                    <IndicatorStopLossFields
                      catalogOptions={catalogOptions}
                      disabled={!settings.indicatorStopLoss.enabled}
                      settings={settings}
                      onSettingsChange={(patch) =>
                        updateRiskSettings("indicatorStopLoss", patch)
                      }
                    />
                  </RiskRuleRow>
                  <Separator className={configSeparatorClassName} />

                  <RiskRuleRow
                    checked={settings.timeStopLoss.enabled}
                    onCheckedChange={(enabled) =>
                      updateRiskSettings("timeStopLoss", { enabled })
                    }
                    title="时间止损"
                  >
                    <div className="grid gap-2 md:grid-cols-[6.5rem_10rem_6.5rem_10rem] md:items-center">
                      <div className="text-xs text-muted-foreground">持仓</div>
                      <CompactNumberInput
                        disabled={!settings.timeStopLoss.enabled}
                        label="持仓"
                        min={1}
                        onValueChange={(holdingDays) =>
                          updateRiskSettings("timeStopLoss", { holdingDays })
                        }
                        step={1}
                        suffix="天"
                        value={settings.timeStopLoss.holdingDays}
                      />
                      <div className="text-xs text-muted-foreground">
                        收益低于
                      </div>
                      <CompactNumberInput
                        disabled={!settings.timeStopLoss.enabled}
                        label="收益低于"
                        onValueChange={(minimumReturnPercent) =>
                          updateRiskSettings("timeStopLoss", {
                            minimumReturnPercent,
                          })
                        }
                        step={0.5}
                        suffix="%"
                        value={settings.timeStopLoss.minimumReturnPercent}
                      />
                    </div>
                  </RiskRuleRow>
                </FieldGroup>
              </FieldSet>
            </CardContent>
          </Card>
        </>
      }
      aside={
        <Card className={cn("h-fit py-0", sectionCardClassName)}>
          <CardHeader>
            <CardTitle>建仓摘要</CardTitle>
            <CardDescription>当前模拟参数</CardDescription>
          </CardHeader>
          <CardContent className="flex flex-col gap-4">
            <SimulationGateState
              backtestValidationError={backtestValidationError}
              isBacktestValidationPending={isBacktestValidationPending}
              isMarketTemplateError={isMarketTemplateError}
              isMarketTemplateLoading={isMarketTemplateLoading}
              marketTemplateError={marketTemplateError}
              previewSnapshot={previewSnapshot}
            />

            <div className="grid grid-cols-2 gap-2">
              <SummaryMetric
                label="初始金额"
                value={formatCurrency(settings.initialCapital)}
              />
              <SummaryMetric
                label="买入信号"
                value={`Top ${settings.buyTopN}`}
              />
              <SummaryMetric label="最大持仓" value={`${maxPositions} 只`} />
              <SummaryMetric
                label="单票上限"
                value={formatCurrency(perPositionCapital)}
              />
              <SummaryMetric
                label="买入规则"
                value={buyRuleLabel}
                valueClassName="whitespace-normal break-words leading-snug"
              />
              <SummaryMetric
                label="调仓规则"
                value={rebalanceRuleLabel}
                valueClassName="whitespace-normal break-words leading-snug"
              />
            </div>

            <Separator />

            <div className="grid grid-cols-2 gap-2">
              <SummaryMetric
                label="佣金率"
                value={formatFeePercent(
                  settings.transactionFees.commissionRatePercent
                )}
              />
              <SummaryMetric
                label="卖出印花税"
                value={formatFeePercent(
                  settings.transactionFees.stampDutyRatePercent
                )}
              />
              <SummaryMetric
                label="过户费"
                value={formatFeePercent(
                  settings.transactionFees.transferFeeRatePercent
                )}
              />
              <SummaryMetric
                label="成交滑点"
                value={formatFeePercent(
                  settings.transactionFees.slippageRatePercent
                )}
              />
            </div>

            <Separator />

            <div className="flex flex-col gap-2">
              <div className="flex items-center justify-between gap-2">
                <div className="text-sm font-medium">近三月票池数</div>
                <Badge variant="secondary">最近 {latestPoolCount} 只</Badge>
              </div>
              <PoolCountTrendChart data={poolCountTrend} />
            </div>

            <Separator />

            <div className="flex flex-col gap-2">
              <div className="text-sm font-medium">卖出条件</div>
              {activeRiskRows.length === 0 ? (
                <div className="text-xs text-muted-foreground">
                  暂无卖出条件。
                </div>
              ) : (
                <Table>
                  <TableHeader>
                    <TableRow className="hover:bg-transparent">
                      <TableHead>条件</TableHead>
                      <TableHead>触发</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {activeRiskRows.map((row) => (
                      <TableRow
                        key={row.condition}
                        className="hover:bg-transparent"
                      >
                        <TableCell className="font-medium">
                          {row.condition}
                        </TableCell>
                        <TableCell className="whitespace-normal text-muted-foreground">
                          {row.trigger}
                        </TableCell>
                      </TableRow>
                    ))}
                  </TableBody>
                </Table>
              )}
            </div>
          </CardContent>
        </Card>
      }
    />
  )
}

function NumberInputField({
  disabled = false,
  label,
  max,
  min,
  onValueChange,
  step,
  suffix,
  value,
}: NumberInputFieldProps) {
  return (
    <Field data-disabled={disabled ? true : undefined}>
      <FieldLabel>{label}</FieldLabel>
      <InputGroup>
        <InputGroupInput
          disabled={disabled}
          inputMode="decimal"
          max={max}
          min={min}
          onChange={(event) =>
            onValueChange(toBoundedNumber(event.target.value, min, max))
          }
          step={step}
          type="number"
          value={String(value)}
        />
        {suffix ? (
          <InputGroupAddon align="inline-end">{suffix}</InputGroupAddon>
        ) : null}
      </InputGroup>
    </Field>
  )
}

function ReadonlySelectField({ label, value }: ReadonlySelectFieldProps) {
  return (
    <Field data-disabled>
      <FieldLabel>{label}</FieldLabel>
      <Select value={value}>
        <SelectTrigger className="w-full bg-background" disabled>
          <SelectValue>{value}</SelectValue>
        </SelectTrigger>
        <SelectContent align="start" className="bg-background">
          <SelectGroup>
            <SelectItem value={value}>{value}</SelectItem>
          </SelectGroup>
        </SelectContent>
      </Select>
    </Field>
  )
}

function CompactNumberInput({
  ariaInvalid = false,
  disabled = false,
  invalid = false,
  label,
  max,
  min,
  onValueChange,
  step,
  suffix,
  value,
}: NumberInputFieldProps) {
  return (
    <Field
      data-disabled={disabled ? true : undefined}
      data-invalid={invalid ? true : undefined}
    >
      <FieldLabel className="sr-only">{label}</FieldLabel>
      <InputGroup className="bg-background">
        <InputGroupInput
          aria-invalid={ariaInvalid}
          aria-label={label}
          disabled={disabled}
          inputMode="decimal"
          max={max}
          min={min}
          onChange={(event) =>
            onValueChange(toBoundedNumber(event.target.value, min, max))
          }
          step={step}
          type="number"
          value={String(value)}
        />
        {suffix ? (
          <InputGroupAddon align="inline-end">{suffix}</InputGroupAddon>
        ) : null}
      </InputGroup>
    </Field>
  )
}

function SettingRow({ children, label }: SettingRowProps) {
  return (
    <div className={settingRowClassName}>
      <div className="min-w-0">{label}</div>
      <div className="min-w-0">{children}</div>
    </div>
  )
}

function TransactionFeeList({
  commissionRateMaxPercent,
  fees,
  isTemplateError,
  isTemplateLoading,
  onRetryTemplate,
  onRateChange,
  templateError,
}: {
  commissionRateMaxPercent: number | null
  fees: SimulationSettings["transactionFees"]
  isTemplateError: boolean
  isTemplateLoading: boolean
  onRetryTemplate: () => void
  onRateChange: (key: TransactionFeeSettingsKey, value: number) => void
  templateError: unknown
}) {
  return (
    <FieldSet>
      <FieldLegend className="sr-only">交易费率</FieldLegend>
      <FieldGroup className={configListClassName}>
        {isTemplateLoading ? (
          <div className="flex flex-col gap-2 py-2">
            <Skeleton className="h-8 w-full" />
            <Skeleton className="h-8 w-4/5" />
          </div>
        ) : null}
        {isTemplateError ? (
          <Alert variant="destructive" className="my-2">
            <AlertTitle>默认市场费率加载失败</AlertTitle>
            <AlertDescription>
              {formatErrorMessage(templateError)}
            </AlertDescription>
            <AlertAction>
              <Button
                size="xs"
                type="button"
                variant="outline"
                onClick={onRetryTemplate}
              >
                重试
              </Button>
            </AlertAction>
          </Alert>
        ) : null}
        {transactionFeeRows.map((row, index) => (
          <div key={row.key}>
            {index > 0 ? (
              <Separator className={configSeparatorClassName} />
            ) : null}
            {row.key === "commissionRatePercent" ? (
              <TransactionFeeRowField
                fee={fees[row.key]}
                isInvalid={
                  commissionRateMaxPercent !== null &&
                  fees[row.key] > commissionRateMaxPercent
                }
                maxRate={commissionRateMaxPercent}
                row={row}
                onRateChange={onRateChange}
              />
            ) : (
              <TransactionFeeRowField
                fee={fees[row.key]}
                isInvalid={false}
                maxRate={null}
                row={row}
                onRateChange={onRateChange}
              />
            )}
          </div>
        ))}
      </FieldGroup>
    </FieldSet>
  )
}

function TransactionFeeRowField({
  fee,
  isInvalid,
  maxRate,
  onRateChange,
  row,
}: {
  fee: number
  isInvalid: boolean
  maxRate: number | null
  onRateChange: (key: TransactionFeeSettingsKey, value: number) => void
  row: TransactionFeeRow
}) {
  const note =
    row.key === "commissionRatePercent" && maxRate !== null
      ? `模板上限 ${formatFeePercent(maxRate)}`
      : row.note

  return (
    <div className={transactionFeeRowClassName}>
      <div className="text-xs font-medium">{row.name}</div>
      <div className="text-xs text-muted-foreground">{row.direction}</div>
      <CompactNumberInput
        ariaInvalid={isInvalid}
        invalid={isInvalid}
        label={`${row.name}费率`}
        min={0}
        onValueChange={(value) => onRateChange(row.key, value)}
        step={row.step}
        suffix="%"
        value={fee}
      />
      <FieldDescription className={cn(isInvalid && "text-destructive")}>
        {isInvalid ? "高于市场模板佣金上限" : note}
      </FieldDescription>
    </div>
  )
}

function RiskRuleRow({
  checked,
  children,
  disabled = false,
  onCheckedChange,
  title,
}: RiskRuleRowProps) {
  return (
    <SettingRow
      label={
        <Field
          className="min-w-0"
          data-disabled={disabled ? true : undefined}
          orientation="horizontal"
        >
          <Checkbox
            aria-label={`启用${title}`}
            checked={checked}
            disabled={disabled}
            onCheckedChange={onCheckedChange}
          />
          <FieldContent>
            <FieldTitle className="text-xs font-medium">{title}</FieldTitle>
          </FieldContent>
        </Field>
      }
    >
      {children}
    </SettingRow>
  )
}

function IndicatorStopLossFields({
  catalogOptions,
  disabled,
  onSettingsChange,
  settings,
}: {
  catalogOptions: IndicatorCatalog[]
  disabled: boolean
  onSettingsChange: (
    patch: Partial<SimulationSettings["indicatorStopLoss"]>
  ) => void
  settings: SimulationSettings
}) {
  const trendMovingAverageCatalogs =
    getTrendMovingAverageCatalogs(catalogOptions)

  if (trendMovingAverageCatalogs.length === 0) {
    return (
      <div className="text-xs text-muted-foreground">暂无可用趋势均线指标</div>
    )
  }

  const selectedCatalog = getCatalog(
    settings.indicatorStopLoss.catalogId,
    trendMovingAverageCatalogs
  )
  const selectedMetrics = getCatalogMetricsByType(
    selectedCatalog.id,
    "number",
    trendMovingAverageCatalogs
  )
  const selectedMetric =
    selectedMetrics.find(
      (metric) => metric.id === settings.indicatorStopLoss.metric
    ) ?? selectedMetrics[0]

  return (
    <div className="grid gap-2 md:grid-cols-[6.5rem_10rem_10rem] md:items-center">
      <div className="text-xs text-muted-foreground">收盘价跌破</div>
      <Field data-disabled={disabled ? true : undefined}>
        <FieldLabel className="sr-only">指标类型</FieldLabel>
        <Select
          value={selectedCatalog.id}
          onValueChange={(catalogId) => {
            if (!catalogId) {
              return
            }

            const metrics = getCatalogMetricsByType(
              catalogId,
              "number",
              trendMovingAverageCatalogs
            )
            const metric = metrics[0]
            if (metric) {
              onSettingsChange({ catalogId, metric: metric.id })
            }
          }}
        >
          <SelectTrigger className="w-full bg-background" disabled={disabled}>
            <SelectValue>
              <span className="truncate">{selectedCatalog.label}</span>
            </SelectValue>
          </SelectTrigger>
          <SelectContent align="start" className="min-w-72 bg-background">
            <SelectGroup>
              <SelectLabel>指标来源</SelectLabel>
              {trendMovingAverageCatalogs.map((catalog) => (
                <SelectItem key={catalog.id} value={catalog.id}>
                  <span className="truncate text-xs font-medium">
                    {catalog.label}
                  </span>
                </SelectItem>
              ))}
            </SelectGroup>
          </SelectContent>
        </Select>
      </Field>

      <Field data-disabled={disabled ? true : undefined}>
        <FieldLabel className="sr-only">止损指标</FieldLabel>
        <Select
          value={selectedMetric?.id ?? settings.indicatorStopLoss.metric}
          onValueChange={(metric) => {
            if (metric) {
              onSettingsChange({ metric })
            }
          }}
        >
          <SelectTrigger className="w-full bg-background" disabled={disabled}>
            <SelectValue>
              <span className="truncate">
                {selectedMetric?.label ?? settings.indicatorStopLoss.metric}
              </span>
            </SelectValue>
          </SelectTrigger>
          <SelectContent align="start" className="min-w-72 bg-background">
            <SelectGroup>
              <SelectLabel>{selectedCatalog.source}</SelectLabel>
              {selectedMetrics.map((metric) => (
                <SelectItem key={metric.id} value={metric.id}>
                  <span className="truncate text-xs">{metric.label}</span>
                </SelectItem>
              ))}
            </SelectGroup>
          </SelectContent>
        </Select>
      </Field>
    </div>
  )
}

function SimulationGateState({
  backtestValidationError,
  isBacktestValidationPending,
  isMarketTemplateError,
  isMarketTemplateLoading,
  marketTemplateError,
  previewSnapshot,
}: {
  backtestValidationError: string | null
  isBacktestValidationPending: boolean
  isMarketTemplateError: boolean
  isMarketTemplateLoading: boolean
  marketTemplateError: unknown
  previewSnapshot: PreviewSnapshot | null
}) {
  if (!previewSnapshot) {
    return (
      <Alert>
        <AlertTitle>需要股池预览</AlertTitle>
        <AlertDescription>
          先在 Step 3 更新股池，Step 4 才能进入回测。
        </AlertDescription>
      </Alert>
    )
  }

  if (previewSnapshot.stale) {
    return (
      <Alert variant="destructive">
        <AlertTitle>股池预览已过期</AlertTitle>
        <AlertDescription>
          Step 1 或 Step 2 已修改，需要回到 Step 3 更新股池后才能进入回测。
        </AlertDescription>
      </Alert>
    )
  }

  if (isMarketTemplateLoading) {
    return (
      <Alert>
        <AlertTitle>正在读取默认费率模板</AlertTitle>
        <AlertDescription>
          默认费用和滑点必须来自 Rearview 市场模板。
        </AlertDescription>
      </Alert>
    )
  }

  if (isMarketTemplateError) {
    return (
      <Alert variant="destructive">
        <AlertTitle>默认费率不可用</AlertTitle>
        <AlertDescription>
          {formatErrorMessage(marketTemplateError)}
        </AlertDescription>
      </Alert>
    )
  }

  if (backtestValidationError) {
    return (
      <Alert variant="destructive">
        <AlertTitle>进入回测失败</AlertTitle>
        <AlertDescription>{backtestValidationError}</AlertDescription>
      </Alert>
    )
  }

  if (isBacktestValidationPending) {
    return (
      <Alert>
        <AlertTitle>正在进入回测</AlertTitle>
        <AlertDescription>Rearview 正在校验当前建仓参数。</AlertDescription>
      </Alert>
    )
  }

  return null
}

function PoolCountTrendChart({ data }: { data: PoolCountTrendPoint[] }) {
  if (data.length === 0) {
    return (
      <div className="flex h-38 items-center text-xs text-muted-foreground">
        暂无票池走势。
      </div>
    )
  }

  const width = 320
  const height = 152
  const padding = {
    top: 12,
    right: 10,
    bottom: 24,
    left: 28,
  }
  const chartWidth = width - padding.left - padding.right
  const chartHeight = height - padding.top - padding.bottom
  const maxCount = Math.max(1, ...data.map((item) => item.count))
  const points = data.map((item, index) => {
    const x = padding.left + (index / Math.max(1, data.length - 1)) * chartWidth
    const y = padding.top + chartHeight - (item.count / maxCount) * chartHeight

    return { ...item, x, y }
  })
  const linePath = points
    .map((point, index) => `${index === 0 ? "M" : "L"} ${point.x} ${point.y}`)
    .join(" ")
  const areaPath = `${linePath} L ${padding.left + chartWidth} ${
    padding.top + chartHeight
  } L ${padding.left} ${padding.top + chartHeight} Z`
  const guideRows = [maxCount, Math.round(maxCount / 2), 0]

  return (
    <div className="py-2">
      <svg
        aria-label="近三个月股票池数量走势"
        className="h-38 w-full"
        role="img"
        viewBox={`0 0 ${width} ${height}`}
      >
        <g className="text-border" stroke="currentColor" strokeWidth="1">
          {guideRows.map((value) => {
            const y =
              padding.top + chartHeight - (value / maxCount) * chartHeight
            return (
              <line
                key={value}
                x1={padding.left}
                x2={padding.left + chartWidth}
                y1={y}
                y2={y}
                vectorEffect="non-scaling-stroke"
              />
            )
          })}
        </g>

        <path className="fill-primary/10" d={areaPath} />
        <path
          className="text-primary"
          d={linePath}
          fill="none"
          stroke="currentColor"
          strokeLinecap="round"
          strokeLinejoin="round"
          strokeWidth="2"
          vectorEffect="non-scaling-stroke"
        />

        {points.map((point, index) =>
          index === points.length - 1 || index % 4 === 0 ? (
            <circle
              key={`${point.label}-${point.count}`}
              className="fill-background text-primary"
              cx={point.x}
              cy={point.y}
              r="3"
              stroke="currentColor"
              strokeWidth="2"
              vectorEffect="non-scaling-stroke"
            />
          ) : null
        )}

        <g className="text-muted-foreground" fill="currentColor">
          {guideRows.map((value) => {
            const y =
              padding.top + chartHeight - (value / maxCount) * chartHeight
            return (
              <text
                key={`label-${value}`}
                dominantBaseline="middle"
                fontSize="10"
                textAnchor="end"
                x={padding.left - 6}
                y={y}
              >
                {value}
              </text>
            )
          })}
          {points.map((point, index) =>
            index === 0 ||
            index === Math.floor(points.length / 2) ||
            index === points.length - 1 ? (
              <text
                key={point.label}
                fontSize="10"
                textAnchor={
                  index === 0
                    ? "start"
                    : index === points.length - 1
                      ? "end"
                      : "middle"
                }
                x={point.x}
                y={height - 6}
              >
                {point.label}
              </text>
            ) : null
          )}
        </g>
      </svg>
    </div>
  )
}

function SummaryMetric({
  className,
  label,
  value,
  valueClassName,
}: {
  className?: string
  label: string
  value: string
  valueClassName?: string
}) {
  return (
    <div className={cn("min-w-0 px-1 py-1", className)}>
      <div className="text-xs text-muted-foreground">{label}</div>
      <div
        className={cn(
          "mt-1 text-sm font-medium tabular-nums",
          valueClassName ?? "truncate"
        )}
      >
        {value}
      </div>
    </div>
  )
}

function buildRiskSummaryRows(settings: SimulationSettings): SummaryRow[] {
  const rows: SummaryRow[] = []
  if (settings.takeProfit.enabled) {
    rows.push({
      condition: "固定止盈",
      trigger: `收益达到 ${formatPercent(settings.takeProfit.profitPercent)}`,
    })
  }

  if (settings.fixedStopLoss.enabled) {
    rows.push({
      condition: "固定止损",
      trigger: `买入价下跌 ${formatPercent(settings.fixedStopLoss.lossPercent)}`,
    })
  }

  if (settings.indicatorStopLoss.enabled) {
    rows.push({
      condition: "指标止损",
      trigger: `收盘价跌破 ${settings.indicatorStopLoss.metric}`,
    })
  }

  if (settings.timeStopLoss.enabled) {
    rows.push({
      condition: "时间止损",
      trigger: `${settings.timeStopLoss.holdingDays} 天后收益低于 ${formatPercent(settings.timeStopLoss.minimumReturnPercent)}`,
    })
  }

  return rows
}

function formatCurrency(value: number) {
  return `¥${Math.round(Math.max(0, value)).toLocaleString("zh-CN")}`
}

function formatPercent(value: number) {
  return `${Number.isInteger(value) ? value : value.toFixed(1)}%`
}

function formatFeePercent(value: number) {
  return `${value.toFixed(3)}%`
}

function formatErrorMessage(error: unknown) {
  if (error instanceof Error) {
    return error.message
  }

  return String(error || "Unknown error")
}

function readSliderValue(
  nextValue: number | readonly number[],
  fallback: number
) {
  const value = Array.isArray(nextValue) ? nextValue[0] : nextValue
  return typeof value === "number" ? value : fallback
}

function toBoundedNumber(value: string, min?: number, max?: number) {
  const parsed = Number(value)
  const fallback = min ?? 0

  if (Number.isNaN(parsed)) {
    return fallback
  }

  return Math.min(
    max ?? Number.POSITIVE_INFINITY,
    Math.max(min ?? parsed, parsed)
  )
}

export { SimulationPositionPanel }
