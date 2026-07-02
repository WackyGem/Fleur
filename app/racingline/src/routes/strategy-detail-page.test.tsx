// @vitest-environment jsdom

import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import { act } from "react"
import { createRoot, type Root } from "react-dom/client"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import { ApiError } from "@/api/client"
import { queryKeys } from "@/api/queryKeys"
import { StrategyDetailPage } from "@/routes/strategy-detail-page"
import type { StrategyPortfolioRecord } from "@/types/rearview"

const mocks = vi.hoisted(() => ({
  navigate: vi.fn(),
  params: { portfolioId: "portfolio-1" } as { portfolioId?: string },
  archiveMutation: {
    isPending: false,
    mutateAsync: vi.fn(),
  },
  portfolioQuery: {
    data: null as StrategyPortfolioRecord | null,
    error: null as unknown,
    isError: false,
    isLoading: false,
  },
  emptyQuery: {
    data: null,
    error: null,
    isError: false,
    isLoading: false,
  },
}))

vi.mock("react-router-dom", () => ({
  Link: ({
    children,
    to,
  }: {
    children: React.ReactNode
    to: string
    viewTransition?: boolean
  }) => <a href={to}>{children}</a>,
  useNavigate: () => mocks.navigate,
  useParams: () => mocks.params,
}))

vi.mock("@/api/hooks", () => ({
  useStrategyPortfolioArchiveMutation: () => mocks.archiveMutation,
  useStrategyPortfolioNavQuery: () => mocks.emptyQuery,
  useStrategyPortfolioPerformanceQuery: () => mocks.emptyQuery,
  useStrategyPortfolioPositionsQuery: () => mocks.emptyQuery,
  useStrategyPortfolioQuery: () => mocks.portfolioQuery,
  useStrategyPortfolioRebalanceRecordsQuery: () => mocks.emptyQuery,
  useStrategyPortfolioSignalsQuery: () => ({
    ...mocks.emptyQuery,
    data: { has_more: false, items: [], pending_buy_signals: [] },
  }),
  useStrategyPortfolioSignalTimelineQuery: () => ({
    ...mocks.emptyQuery,
    data: { trade_dates: [] },
  }),
  useStrategyPortfolioStatementQuery: () => mocks.emptyQuery,
  useStrategyPortfolioVirtualAccountQuery: () => mocks.emptyQuery,
}))

vi.mock("lightweight-charts", () => ({
  createChart: () => ({
    addSeries: () => ({ setData: vi.fn() }),
    applyOptions: vi.fn(),
    remove: vi.fn(),
    subscribeClick: vi.fn(),
    timeScale: () => ({ fitContent: vi.fn() }),
    unsubscribeClick: vi.fn(),
  }),
  LineSeries: {},
  TickMarkType: {
    DayOfMonth: 0,
    Month: 1,
    Year: 2,
  },
}))

describe("StrategyDetailPage delete action", () => {
  let container: HTMLDivElement
  let root: Root
  let queryClient: QueryClient

  beforeEach(() => {
    ;(
      globalThis as typeof globalThis & {
        IS_REACT_ACT_ENVIRONMENT: boolean
      }
    ).IS_REACT_ACT_ENVIRONMENT = true
    globalThis.ResizeObserver = class ResizeObserver {
      disconnect() {}
      observe() {}
      unobserve() {}
    }
    container = document.createElement("div")
    document.body.appendChild(container)
    root = createRoot(container)
    queryClient = new QueryClient({
      defaultOptions: { queries: { retry: false } },
    })
    vi.spyOn(queryClient, "invalidateQueries").mockResolvedValue()
    vi.spyOn(queryClient, "removeQueries")
    mocks.navigate.mockReset()
    mocks.archiveMutation.isPending = false
    mocks.archiveMutation.mutateAsync.mockReset()
    mocks.archiveMutation.mutateAsync.mockResolvedValue(portfolioRecord())
    mocks.portfolioQuery.data = portfolioRecord()
    mocks.portfolioQuery.error = null
    mocks.portfolioQuery.isError = false
    mocks.portfolioQuery.isLoading = false
  })

  afterEach(() => {
    act(() => {
      root.unmount()
    })
    queryClient.clear()
    document.body.innerHTML = ""
    vi.restoreAllMocks()
  })

  it("archives the portfolio and navigates to dashboard after confirmation", async () => {
    await renderPage()

    await clickButton("删除")
    await clickButton("确认删除")

    expect(mocks.archiveMutation.mutateAsync).toHaveBeenCalledWith(
      "portfolio-1"
    )
    expect(queryClient.invalidateQueries).toHaveBeenCalledWith({
      queryKey: queryKeys.strategyPortfolioDashboard(),
    })
    expect(queryClient.removeQueries).toHaveBeenCalledWith({
      queryKey: queryKeys.strategyPortfolio("portfolio-1"),
    })
    expect(mocks.navigate).toHaveBeenCalledWith("/dashboard", {
      viewTransition: true,
    })
  })

  it("keeps the delete dialog open and shows the backend error on failure", async () => {
    mocks.archiveMutation.mutateAsync.mockRejectedValue(
      new ApiError(500, { message: "archive failed" }, "fallback")
    )
    await renderPage()

    await clickButton("删除")
    await clickButton("确认删除")

    expect(document.body.textContent).toContain("删除失败")
    expect(document.body.textContent).toContain("archive failed")
    expect(mocks.navigate).not.toHaveBeenCalled()
  })

  it("redirects archived portfolio detail errors to dashboard", async () => {
    mocks.portfolioQuery.data = null
    mocks.portfolioQuery.error = new ApiError(
      410,
      { error_type: "gone" },
      "gone"
    )
    mocks.portfolioQuery.isError = true

    await renderPage()

    expect(mocks.navigate).toHaveBeenCalledWith("/dashboard", {
      replace: true,
      viewTransition: true,
    })
    expect(document.body.textContent).toContain("策略已删除")
  })

  it("does not use archived redirect behavior for not found errors", async () => {
    mocks.portfolioQuery.data = null
    mocks.portfolioQuery.error = new ApiError(
      404,
      { error_type: "not_found" },
      "not found"
    )
    mocks.portfolioQuery.isError = true

    await renderPage()

    expect(mocks.navigate).not.toHaveBeenCalled()
    expect(document.body.textContent).toContain("策略不存在或链接无效。")
  })

  async function renderPage() {
    await act(async () => {
      root.render(
        <QueryClientProvider client={queryClient}>
          <StrategyDetailPage />
        </QueryClientProvider>
      )
    })
  }

  async function clickButton(label: string) {
    const button = Array.from(document.body.querySelectorAll("button")).find(
      (candidate) => candidate.textContent?.includes(label)
    )

    if (!button) {
      throw new Error(`button not found: ${label}`)
    }

    await act(async () => {
      button.dispatchEvent(new MouseEvent("click", { bubbles: true }))
    })
  }
})

function portfolioRecord(): StrategyPortfolioRecord {
  return {
    archived_at: null,
    backtest_segment: {
      benchmark_security_code: "000300.SH",
      end_date: "2026-06-26",
      period_key: "1y",
      source_result_attempt_id: "attempt-1",
      source_strategy_backtest_run_id: "backtest-1",
      start_date: "2025-06-26",
    },
    benchmark_security_code: "000300.SH",
    catalog_hash: "catalog-hash",
    client_request_id: "request-1",
    created_at: "2026-06-27T00:00:00Z",
    current_live_result_attempt_id: null,
    current_result_attempt_id: null,
    execution_config: {
      account: { currency: "CNY", initial_cash: 1000000 },
      fee_profile: {
        commission_rate: 0.0003,
        commission_rate_max: 0.0003,
        min_commission: 5,
        stamp_duty_rate_sell: 0.001,
        transfer_fee_rate: 0.00001,
      },
      market: "CN_A_SHARE",
      price_basis: "backward_adjusted",
      rebalance_policy: {
        cash_reserve_pct: 0.02,
        empty_signal_action: "hold",
        lot_size: 100,
        max_positions: 5,
        min_trade_lots: 1,
        single_position_limit_pct: 0.2,
        target_weighting: "equal_weight_capped",
      },
      risk_exit_policy: {
        exit_rules: [],
        trigger_timing: "close_confirm_next_open",
      },
      signal_policy: {
        buy_signal_top_n: 5,
        signal_timing: "close_confirm_next_open",
      },
      slippage_profile: { buy_bps: 5, mode: "bps", sell_bps: 5 },
    },
    execution_config_hash: "execution-hash",
    initial_signal_date: "2026-06-26",
    latest_daily_run_id: null,
    live_segment: {
      current_live_result_attempt_id: null,
      initial_signal_date: "2026-06-26",
      latest_daily_run_id: null,
      live_start_date: "2026-06-29",
      live_status: "pending_first_run",
      performance_source: "none",
      signal_source: "publish_preview",
    },
    live_start_date: "2026-06-29",
    live_status: "pending_first_run",
    name: "低位反转组合",
    pending_buy_signal_snapshot: [],
    portfolio_code: "PF-20260627-0001",
    price_basis: "backward_adjusted",
    request_hash: "request-hash",
    required_marts: [],
    required_metrics: [],
    rule_hash: "rule-hash",
    rule_snapshot: {},
    source_end_date: "2026-06-26",
    source_period_key: "1y",
    source_result_attempt_id: "attempt-1",
    source_start_date: "2025-06-26",
    source_strategy_backtest_run_id: "backtest-1",
    status: "active",
    strategy_portfolio_id: "portfolio-1",
    ui_display_snapshot: {},
    updated_at: "2026-06-27T00:00:00Z",
  }
}
