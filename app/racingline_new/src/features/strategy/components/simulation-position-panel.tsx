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
import type { BacktestExecutionDraft } from "@/features/strategy/execution"
import type { PreviewSnapshot } from "@/features/strategy/preview"
import type {
  SimulationSettings,
  StrategyConditionGroup,
  WeightIndicator,
} from "@/features/strategy/types"
import { getScaledWeightIndicators } from "@/features/strategy/utils"
import { cn } from "@/lib/utils"

type SimulationPositionPanelProps = {
  appliedWeightIndicators: WeightIndicator[]
  backtestValidationError: string | null
  conditionGroups: StrategyConditionGroup[]
  executionDraft: BacktestExecutionDraft | null
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
  disabled?: boolean
  label: string
  max?: number
  min?: number
  onValueChange: (value: number) => void
  step?: number
  suffix?: string
  value: number
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
  max?: number
  min?: number
  name: string
  note: string
  step: number
  suffix: string
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

const transactionFeeRows: TransactionFeeRow[] = [
  {
    direction: "卖出",
    key: "stampDutyRatePercent",
    name: "印花税",
    note: "",
    step: 0.001,
    suffix: "%",
  },
  {
    direction: "双向",
    key: "transferFeeRatePercent",
    name: "过户费",
    note: "",
    step: 0.001,
    suffix: "%",
  },
  {
    direction: "双向",
    key: "commissionRatePercent",
    name: "佣金",
    note: "",
    step: 0.001,
    suffix: "%",
  },
  {
    direction: "双向",
    key: "commissionRateMaxPercent",
    name: "佣金上限",
    note: "单笔佣金率封顶",
    step: 0.001,
    suffix: "%",
  },
  {
    direction: "双向",
    key: "minCommission",
    min: 0,
    name: "最低佣金",
    note: "按单笔成交金额计",
    step: 0.1,
    suffix: "元",
  },
  {
    direction: "买入",
    key: "buySlippageRatePercent",
    name: "买入滑点",
    note: "买入按参考价上浮",
    step: 0.001,
    suffix: "%",
  },
  {
    direction: "双向",
    key: "sellSlippageRatePercent",
    name: "卖出滑点",
    note: "卖出按参考价下浮",
    step: 0.001,
    suffix: "%",
  },
]

function SimulationPositionPanel({
  appliedWeightIndicators,
  backtestValidationError,
  conditionGroups,
  executionDraft,
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
  const executionSummary = executionDraft?.summary
  const targetWeight = executionSummary?.target_weight_per_position_pct ?? null
  const implicitCashReserve =
    executionSummary?.implicit_cash_reserve_pct ?? null
  const perPositionCapital =
    targetWeight === null ? null : settings.initialCapital * targetWeight
  const maxPositions = executionSummary?.max_positions ?? settings.buyTopN
  const groupCount = conditionGroups.length
  const conditionCount = conditionGroups.reduce(
    (total, group) => total + group.conditions.length,
    0
  )
  const { indicators } = getScaledWeightIndicators(appliedWeightIndicators)
  const previewStatus = previewSnapshot
    ? previewSnapshot.stale
      ? "已过期"
      : "可用"
    : "未生成"

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
                <FieldGroup className="grid max-w-[48rem] gap-3 md:grid-cols-[12rem_12rem_19rem]">
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
                    label="买入信号 Top N"
                    min={1}
                    onValueChange={(buyTopN) => updateSettings({ buyTopN })}
                    step={1}
                    suffix="只"
                    value={settings.buyTopN}
                  />

                  <Field>
                    <FieldLabel>单票上限</FieldLabel>
                    <div className="grid grid-cols-[minmax(0,1fr)_8rem] items-center gap-3">
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
                    checked={false}
                    disabled
                    onCheckedChange={() => undefined}
                    title="指标止损"
                  >
                    <div className="text-xs text-muted-foreground">
                      Rearview 当前只开放固定止损、固定止盈和时间止损，指标止损暂不进入回测草稿。
                    </div>
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
            <CardDescription>Rearview 回测草稿</CardDescription>
          </CardHeader>
          <CardContent className="flex flex-col gap-4">
            <BacktestDraftState
              backtestValidationError={backtestValidationError}
              executionDraft={executionDraft}
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
                label="账户币种"
                value={executionDraft?.execution_config.account.currency ?? "CNY"}
              />
              <SummaryMetric
                label="买入信号"
                value={`Top ${settings.buyTopN}`}
              />
              <SummaryMetric
                label="最大持仓"
                value={`${maxPositions} 只`}
              />
              <SummaryMetric
                label="单票目标"
                value={
                  targetWeight === null
                    ? "待校验"
                    : formatDecimalPercent(targetWeight)
                }
              />
              <SummaryMetric
                label="单票金额"
                value={
                  perPositionCapital === null
                    ? "待校验"
                    : formatCurrency(perPositionCapital)
                }
              />
              <SummaryMetric
                label="现金保留"
                value={
                  implicitCashReserve === null
                    ? "待校验"
                    : formatDecimalPercent(implicitCashReserve)
                }
              />
              <SummaryMetric
                label="卖出规则"
                value={`${executionSummary?.enabled_exit_rule_count ?? activeRiskRows.length} 条`}
              />
            </div>

            <Separator />

            <div className="grid grid-cols-2 gap-2">
              <SummaryMetric
                label="Preview"
                value={previewSnapshot?.previewId ?? "未生成"}
              />
              <SummaryMetric label="Preview 状态" value={previewStatus} />
              <SummaryMetric
                label="Preview 区间"
                value={
                  previewSnapshot
                    ? `${previewSnapshot.range.startDate} 至 ${previewSnapshot.range.endDate}`
                    : "未生成"
                }
              />
              <SummaryMetric
                label="草稿 Hash"
                value={
                  executionDraft
                    ? compactHash(executionDraft.execution_config_hash)
                    : "待校验"
                }
              />
              <SummaryMetric label="指标组" value={`${groupCount} 组`} />
              <SummaryMetric label="选股条件" value={`${conditionCount} 条`} />
              <SummaryMetric
                label="权重指标"
                value={`${indicators.length} 条`}
              />
            </div>

            <Separator />

            <div className="grid grid-cols-2 gap-2">
              <SummaryMetric
                label="佣金率"
                value={formatPercent(settings.transactionFees.commissionRatePercent)}
              />
              <SummaryMetric
                label="佣金上限"
                value={formatPercent(
                  settings.transactionFees.commissionRateMaxPercent
                )}
              />
              <SummaryMetric
                label="最低佣金"
                value={formatCurrency(settings.transactionFees.minCommission)}
              />
              <SummaryMetric
                label="卖出印花税"
                value={formatPercent(settings.transactionFees.stampDutyRatePercent)}
              />
              <SummaryMetric
                label="过户费"
                value={formatPercent(settings.transactionFees.transferFeeRatePercent)}
              />
              <SummaryMetric
                label="买入滑点"
                value={formatPercent(
                  settings.transactionFees.buySlippageRatePercent
                )}
              />
              <SummaryMetric
                label="卖出滑点"
                value={formatPercent(
                  settings.transactionFees.sellSlippageRatePercent
                )}
              />
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

function CompactNumberInput({
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
      <FieldLabel className="sr-only">{label}</FieldLabel>
      <InputGroup className="bg-background">
        <InputGroupInput
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
  fees,
  isTemplateError,
  isTemplateLoading,
  onRetryTemplate,
  onRateChange,
  templateError,
}: {
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
            <div className={transactionFeeRowClassName}>
              <div className="text-xs font-medium">{row.name}</div>
              <div className="text-xs text-muted-foreground">
                {row.direction}
              </div>
              <CompactNumberInput
                label={`${row.name}费率`}
                max={row.max}
                min={row.min ?? 0}
                onValueChange={(value) => onRateChange(row.key, value)}
                step={row.step}
                suffix={row.suffix}
                value={fees[row.key]}
              />
              <div className="text-xs text-muted-foreground">{row.note}</div>
            </div>
          </div>
        ))}
      </FieldGroup>
    </FieldSet>
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

function BacktestDraftState({
  backtestValidationError,
  executionDraft,
  isBacktestValidationPending,
  isMarketTemplateError,
  isMarketTemplateLoading,
  marketTemplateError,
  previewSnapshot,
}: {
  backtestValidationError: string | null
  executionDraft: BacktestExecutionDraft | null
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
          先在 Step 3 更新股池，Step 4 才能生成回测执行草稿。
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
        <AlertDescription>{formatErrorMessage(marketTemplateError)}</AlertDescription>
      </Alert>
    )
  }

  if (backtestValidationError) {
    return (
      <Alert variant="destructive">
        <AlertTitle>回测草稿校验失败</AlertTitle>
        <AlertDescription>{backtestValidationError}</AlertDescription>
      </Alert>
    )
  }

  if (isBacktestValidationPending) {
    return (
      <Alert>
        <AlertTitle>正在生成回测草稿</AlertTitle>
        <AlertDescription>
          Rearview 正在校验 canonical config 和执行参数 hash。
        </AlertDescription>
      </Alert>
    )
  }

  if (!executionDraft) {
    return (
      <Alert>
        <AlertTitle>待生成回测草稿</AlertTitle>
        <AlertDescription>
          修改参数后需要等待 Rearview 返回新的 canonical draft。
        </AlertDescription>
      </Alert>
    )
  }

  return (
    <div className="flex items-center justify-between gap-2">
      <div className="text-xs text-muted-foreground">
        规则和建仓参数已由 Rearview 校验
      </div>
      <Badge variant="secondary">Draft ready</Badge>
    </div>
  )
}

function SummaryMetric({ label, value }: { label: string; value: string }) {
  return (
    <div className="min-w-0 px-1 py-1">
      <div className="text-xs text-muted-foreground">{label}</div>
      <div className="mt-1 truncate text-sm font-medium tabular-nums">
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

function formatDecimalPercent(value: number) {
  return `${Number((value * 100).toFixed(2))}%`
}

function compactHash(value: string) {
  return value.length <= 12 ? value : `${value.slice(0, 8)}...${value.slice(-4)}`
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
