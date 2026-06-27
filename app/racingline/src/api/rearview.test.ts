import { afterEach, describe, expect, it, vi } from "vitest"

import {
  createStrategyPortfolio,
  getStrategyPortfolioPublishPreview,
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
})

function jsonResponse(body: unknown) {
  return new Response(JSON.stringify(body), {
    headers: { "Content-Type": "application/json" },
    status: 200,
  })
}
