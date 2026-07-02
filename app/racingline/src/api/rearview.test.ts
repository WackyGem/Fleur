import { afterEach, describe, expect, it, vi } from "vitest"

import {
  createStrategyPortfolio,
  getStrategyPortfolioPublishPreview,
  getStrategyPortfolioStatement,
  getStrategyPortfolioVirtualAccount,
} from "@/api/rearview"

describe("strategy portfolio API", () => {
  afterEach(() => {
    vi.unstubAllGlobals()
  })

  it("requests publish preview with the selected result attempt", async () => {
    const fetchMock = vi.fn().mockResolvedValue(
      jsonResponse({
        benchmark_security_code: "000300.SH",
        blockers: [],
        can_publish: true,
        pending_buy_signals: [],
        planned_live_start_date: "2026-06-29",
        publish_cutoff_time: "15:00:00+08:00",
        required_source_signal_date: "2026-06-26",
        server_current_date: "2026-06-26",
        server_current_time: "14:30:00+08:00",
        market_phase: "before_close",
        source_end_date: "2026-06-26",
        source_period_key: "1y",
        source_result_attempt_id: "attempt-1",
        source_signal_date: "2026-06-26",
        source_start_date: "2025-06-26",
        source_strategy_backtest_run_id: "run-1",
      })
    )
    vi.stubGlobal("fetch", fetchMock)

    await getStrategyPortfolioPublishPreview("run-1", "attempt-1")

    expect(fetchMock.mock.calls[0]?.[0]).toBe(
      "http://127.0.0.1:34057/rearview/strategy-backtests/run-1/portfolio-publish-preview?source_result_attempt_id=attempt-1"
    )
  })

  it("sends expected signal and live start dates when creating a portfolio", async () => {
    const fetchMock = vi
      .fn()
      .mockResolvedValue(jsonResponse({ strategy_portfolio_id: "portfolio-1" }))
    vi.stubGlobal("fetch", fetchMock)
    const request = {
      client_request_id: "portfolio-create-1",
      expected_live_start_date: "2026-06-29",
      expected_required_source_signal_date: "2026-06-26",
      expected_source_signal_date: "2026-06-26",
      name: "低位反转组合",
      source_result_attempt_id: "attempt-1",
      source_strategy_backtest_run_id: "run-1",
    }

    await createStrategyPortfolio(request)

    expect(fetchMock).toHaveBeenCalledWith(
      "http://127.0.0.1:34057/rearview/strategy-portfolios",
      expect.objectContaining({
        body: JSON.stringify(request),
        method: "POST",
      })
    )
  })

  it("requests strategy portfolio virtual account by portfolio id", async () => {
    const fetchMock = vi.fn().mockResolvedValue(
      jsonResponse({
        account_date: "2026-06-27",
        cash_balance: 200_000,
        currency: "CNY",
        daily_pnl: -2_345.67,
        daily_return: -0.0023,
        holding_unrealized_pnl: 12_345.67,
        position_count: 5,
        position_market_value: 812_345.67,
        result_attempt_id: "attempt-1",
        source: "live_daily_run",
        strategy_portfolio_daily_run_id: "daily-run-1",
        strategy_portfolio_id: "portfolio-1",
        total_equity: 1_012_345.67,
      })
    )
    vi.stubGlobal("fetch", fetchMock)

    await getStrategyPortfolioVirtualAccount("portfolio-1")

    expect(fetchMock.mock.calls[0]?.[0]).toBe(
      "http://127.0.0.1:34057/rearview/strategy-portfolios/portfolio-1/virtual-account"
    )
  })

  it("requests strategy portfolio statement with period and page", async () => {
    const fetchMock = vi.fn().mockResolvedValue(
      jsonResponse({
        operations: { has_more: false, items: [], limit: 100, offset: 20 },
        period: {
          end_date: "2026-06-26",
          key: "three_months",
          label: "近三月",
          latest_live_trade_date: "2026-06-26",
          start_date: "2026-03-26",
        },
        result_attempt_id: "attempt-1",
        source: "live_daily_run",
        strategy_portfolio_daily_run_id: "daily-run-1",
        strategy_portfolio_id: "portfolio-1",
        summary: {
          average_position_pct: 0.72,
          holding_days: 57,
          losing_security_count: 2,
          trade_count: 18,
          trade_win_rate: 0.5,
          traded_security_count: 12,
          winning_security_count: 5,
        },
      })
    )
    vi.stubGlobal("fetch", fetchMock)

    await getStrategyPortfolioStatement("portfolio-1", {
      limit: 100,
      offset: 20,
      period: "three_months",
    })

    expect(fetchMock.mock.calls[0]?.[0]).toBe(
      "http://127.0.0.1:34057/rearview/strategy-portfolios/portfolio-1/statement?limit=100&offset=20&period=three_months"
    )
  })
})

function jsonResponse(body: unknown) {
  return new Response(JSON.stringify(body), {
    headers: { "Content-Type": "application/json" },
    status: 200,
  })
}
