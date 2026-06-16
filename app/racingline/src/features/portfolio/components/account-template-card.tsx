import { useMemo, useState } from "react"

import {
  useAccountTemplatesQuery,
  useCreateAccountTemplateMutation,
  useDefaultMarketFeeTemplateQuery,
  useUpdateAccountTemplateMutation,
} from "@/api/hooks"
import { ErrorState } from "@/components/racingline/data-state"
import { Button } from "@/components/ui/button"
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card"
import {
  Field,
  FieldDescription,
  FieldGroup,
  FieldLabel,
} from "@/components/ui/field"
import { Input } from "@/components/ui/input"
import { shortId } from "@/lib/format"
import type {
  AccountTemplateRecord,
  CreateAccountTemplateRequest,
  FeeProfile,
  JsonRecord,
  MarketFeeTemplateRecord,
  PatchAccountTemplateRequest,
  SlippageProfile,
} from "@/types/rearview"

export function AccountTemplateCard({
  ruleSetId,
  topN,
}: {
  ruleSetId: string
  topN: number
}) {
  const templatesQuery = useAccountTemplatesQuery(ruleSetId)
  const defaultTemplateQuery = useDefaultMarketFeeTemplateQuery()
  const createMutation = useCreateAccountTemplateMutation(ruleSetId)
  const updateMutation = useUpdateAccountTemplateMutation(ruleSetId)
  const template = useMemo(
    () =>
      templatesQuery.data?.find((item) => item.is_default) ??
      templatesQuery.data?.[0],
    [templatesQuery.data]
  )
  const initialForm = useMemo(
    () => accountTemplateInitialForm(template, defaultTemplateQuery.data, topN),
    [defaultTemplateQuery.data, template, topN]
  )
  const formKey =
    template?.account_template_id ??
    defaultTemplateQuery.data?.market_fee_template_id ??
    `new-${topN}`

  return (
    <Card>
      <CardHeader>
        <CardTitle>Virtual account template</CardTitle>
        <CardDescription>
          {template
            ? `${shortId(template.account_template_id)} / ${template.currency}`
            : "CN_A_SHARE defaults"}
        </CardDescription>
      </CardHeader>
      <CardContent>
        {templatesQuery.isError ? (
          <ErrorState
            error={templatesQuery.error}
            title="Account template API returned an error"
          />
        ) : null}
        {defaultTemplateQuery.isError ? (
          <ErrorState
            error={defaultTemplateQuery.error}
            title="Default market template API returned an error"
          />
        ) : null}
        <AccountTemplateForm
          key={formKey}
          defaultMarketTemplate={defaultTemplateQuery.data}
          disabled={
            templatesQuery.isFetching ||
            defaultTemplateQuery.isFetching ||
            createMutation.isPending ||
            updateMutation.isPending
          }
          initialForm={initialForm}
          onCreate={(request) =>
            createMutation.mutateAsync({
              targetRuleSetId: ruleSetId,
              request,
            })
          }
          onUpdate={(accountTemplateId, request) =>
            updateMutation.mutateAsync({ accountTemplateId, request })
          }
          template={template}
          topN={topN}
        />
      </CardContent>
    </Card>
  )
}

type AccountTemplateFormState = {
  initialCash: string
  maxPositions: string
  commissionRate: string
  minCommission: string
  buyBps: string
  sellBps: string
  stopLossPct: string
  takeProfitPct: string
}

function AccountTemplateForm({
  defaultMarketTemplate,
  disabled,
  initialForm,
  onCreate,
  onUpdate,
  template,
  topN,
}: {
  defaultMarketTemplate?: MarketFeeTemplateRecord
  disabled: boolean
  initialForm: AccountTemplateFormState
  onCreate: (request: CreateAccountTemplateRequest) => Promise<unknown>
  onUpdate: (
    accountTemplateId: string,
    request: PatchAccountTemplateRequest
  ) => Promise<unknown>
  template?: AccountTemplateRecord
  topN: number
}) {
  const [form, setForm] = useState(initialForm)

  function updateField(key: keyof AccountTemplateFormState, value: string) {
    setForm((current) => ({ ...current, [key]: value }))
  }

  async function saveTemplate() {
    const baseFee = template?.fee_profile ?? defaultMarketTemplate?.fee_profile
    const baseSlippage =
      template?.slippage_profile ?? defaultMarketTemplate?.slippage_profile
    const feeProfile: FeeProfile = {
      commission_rate: numberInput(form.commissionRate, 0.0001),
      commission_rate_max: baseFee?.commission_rate_max ?? 0.003,
      min_commission: numberInput(form.minCommission, 5),
      stamp_duty_rate_sell: baseFee?.stamp_duty_rate_sell ?? 0.0005,
      transfer_fee_rate: baseFee?.transfer_fee_rate ?? 0.00001,
    }
    const slippageProfile: SlippageProfile = {
      mode: baseSlippage?.mode ?? "bps",
      buy_bps: numberInput(form.buyBps, 10),
      sell_bps: numberInput(form.sellBps, 10),
    }
    const rebalancePolicy: JsonRecord = {
      ...(template?.rebalance_policy ?? {}),
      frequency: "signal_day",
      target_weighting: "equal_weight",
      max_positions: numberInput(form.maxPositions, topN || 10),
      lot_size: 100,
      min_trade_lots: 1,
      cash_reserve_pct: 0,
      empty_signal_action: "hold",
    }
    const riskExitPolicy: JsonRecord = {
      trigger_timing: "close_confirm_next_open",
      exit_rules: exitRulesFromForm(form.stopLossPct, form.takeProfitPct),
    }
    if (!template) {
      await onCreate({
        market: "CN_A_SHARE",
        name: "Default research account",
        initial_cash: numberInput(form.initialCash, 1_000_000),
        fee_profile: feeProfile,
        slippage_profile: slippageProfile,
        rebalance_policy: rebalancePolicy,
        risk_exit_policy: riskExitPolicy,
        is_default: true,
      })
      return
    }
    await onUpdate(template.account_template_id, {
      initial_cash: numberInput(form.initialCash, 1_000_000),
      fee_profile: feeProfile,
      slippage_profile: slippageProfile,
      rebalance_policy: rebalancePolicy,
      risk_exit_policy: riskExitPolicy,
      is_default: true,
    })
  }

  return (
    <FieldGroup>
      <div className="grid gap-3 md:grid-cols-3">
        <NumberField
          label="Initial cash"
          onChange={(value) => updateField("initialCash", value)}
          value={form.initialCash}
        />
        <NumberField
          label="Max positions"
          onChange={(value) => updateField("maxPositions", value)}
          value={form.maxPositions}
        />
        <NumberField
          label="Commission rate"
          onChange={(value) => updateField("commissionRate", value)}
          value={form.commissionRate}
        />
        <NumberField
          label="Min commission"
          onChange={(value) => updateField("minCommission", value)}
          value={form.minCommission}
        />
        <NumberField
          label="Buy bps"
          onChange={(value) => updateField("buyBps", value)}
          value={form.buyBps}
        />
        <NumberField
          label="Sell bps"
          onChange={(value) => updateField("sellBps", value)}
          value={form.sellBps}
        />
        <NumberField
          label="Stop loss pct"
          onChange={(value) => updateField("stopLossPct", value)}
          placeholder="0.08"
          value={form.stopLossPct}
        />
        <NumberField
          label="Take profit pct"
          onChange={(value) => updateField("takeProfitPct", value)}
          placeholder="0.15"
          value={form.takeProfitPct}
        />
      </div>
      <FieldDescription>
        Fee defaults come from the active CN_A_SHARE market template.
      </FieldDescription>
      <Button
        disabled={disabled}
        onClick={() => void saveTemplate()}
        size="sm"
        variant="outline"
      >
        Save account template
      </Button>
    </FieldGroup>
  )
}

function NumberField({
  label,
  value,
  onChange,
  placeholder,
}: {
  label: string
  value: string
  onChange: (value: string) => void
  placeholder?: string
}) {
  return (
    <Field>
      <FieldLabel>{label}</FieldLabel>
      <Input
        inputMode="decimal"
        onChange={(event) => onChange(event.currentTarget.value)}
        placeholder={placeholder}
        value={value}
      />
    </Field>
  )
}

function accountTemplateInitialForm(
  template: AccountTemplateRecord | undefined,
  defaultMarketTemplate: MarketFeeTemplateRecord | undefined,
  topN: number
): AccountTemplateFormState {
  const feeProfile = template?.fee_profile ?? defaultMarketTemplate?.fee_profile
  const slippageProfile =
    template?.slippage_profile ?? defaultMarketTemplate?.slippage_profile
  return {
    initialCash: String(template?.initial_cash ?? 1_000_000),
    maxPositions: String(
      numberField(template?.rebalance_policy, "max_positions") ?? topN ?? 10
    ),
    commissionRate: String(feeProfile?.commission_rate ?? 0.0001),
    minCommission: String(feeProfile?.min_commission ?? 5),
    buyBps: String(slippageProfile?.buy_bps ?? 10),
    sellBps: String(slippageProfile?.sell_bps ?? 10),
    stopLossPct: String(
      exitRulePct(template?.risk_exit_policy, "fixed_stop_loss", "loss_pct") ??
        ""
    ),
    takeProfitPct: String(
      exitRulePct(template?.risk_exit_policy, "take_profit", "profit_pct") ?? ""
    ),
  }
}

function numberInput(value: string, fallback: number) {
  const parsed = Number(value)
  return Number.isFinite(parsed) ? parsed : fallback
}

function numberField(record: JsonRecord | undefined, key: string) {
  const value = record?.[key]
  return typeof value === "number" ? value : undefined
}

function exitRulePct(
  riskExitPolicy: JsonRecord | undefined,
  type: string,
  key: string
) {
  const exitRules = riskExitPolicy?.exit_rules
  if (!Array.isArray(exitRules)) {
    return undefined
  }
  const rule = exitRules.find(
    (item): item is JsonRecord => isJsonRecord(item) && item.type === type
  )
  const value = rule?.[key]
  return typeof value === "number" ? value : undefined
}

function isJsonRecord(value: unknown): value is JsonRecord {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value)
}

function exitRulesFromForm(stopLossPct: string, takeProfitPct: string) {
  const rules: JsonRecord[] = []
  const stopLoss = Number(stopLossPct)
  if (Number.isFinite(stopLoss) && stopLoss > 0) {
    rules.push({ type: "fixed_stop_loss", loss_pct: stopLoss })
  }
  const takeProfit = Number(takeProfitPct)
  if (Number.isFinite(takeProfit) && takeProfit > 0) {
    rules.push({ type: "take_profit", profit_pct: takeProfit })
  }
  return rules
}
