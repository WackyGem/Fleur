import type { BacktestExecutionDraft, BacktestPeriodValue } from "./execution"
import type {
  StrategyBacktestRunRecord,
  StrategyBacktestRunStatus,
  StrategyBacktestRunStatusView,
} from "@/types/rearview"

export function acceptStrategyBacktestRunForStep5(
  run: StrategyBacktestRunRecord
) {
  return {
    activeRun: run,
    activeStep: "backtest" as const,
  }
}

export function isStrategyBacktestFailedStatus(
  status: StrategyBacktestRunStatus
) {
  return status.startsWith("failed_")
}

export function isStrategyBacktestTerminalStatus(
  status: StrategyBacktestRunStatus
): boolean {
  return (
    status === "succeeded" ||
    status === "cancelled" ||
    isStrategyBacktestFailedStatus(status)
  )
}

export function mergeStrategyBacktestStatus(
  run: StrategyBacktestRunRecord,
  status: StrategyBacktestRunStatusView
): StrategyBacktestRunRecord {
  if (run.strategy_backtest_run_id !== status.strategy_backtest_run_id) {
    return run
  }

  return {
    ...run,
    status: status.status,
    dispatch_status: status.dispatch_status,
    progress: status.progress,
    error_type: status.error_type,
    error_message: status.error_message,
    period_key: status.period_key,
    benchmark_security_code: status.benchmark_security_code,
    start_date: status.start_date,
    end_date: status.end_date,
    rule_hash: status.rule_hash,
    execution_config_hash: status.execution_config_hash,
    current_result_attempt_id: status.current_result_attempt_id,
  }
}

export function hasStrategyBacktestConfigChanged(
  run: StrategyBacktestRunStatusView | StrategyBacktestRunRecord | null,
  draft: BacktestExecutionDraft | null,
  period: BacktestPeriodValue,
  benchmark: string
) {
  if (!run || !draft) {
    return false
  }

  return (
    run.period_key !== period ||
    run.benchmark_security_code !== benchmark ||
    run.rule_hash !== draft.rule_hash ||
    run.execution_config_hash !== draft.execution_config_hash
  )
}

export function isStrategyBacktestResultReady(
  run: StrategyBacktestRunStatusView | StrategyBacktestRunRecord | null,
  draft: BacktestExecutionDraft | null,
  period: BacktestPeriodValue,
  benchmark: string
) {
  return Boolean(
    run?.status === "succeeded" &&
      run.current_result_attempt_id &&
      !hasStrategyBacktestConfigChanged(run, draft, period, benchmark)
  )
}
