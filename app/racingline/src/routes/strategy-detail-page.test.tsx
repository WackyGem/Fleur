// @vitest-environment jsdom

import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import { act } from "react"
import { createRoot, type Root } from "react-dom/client"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import { ApiError } from "@/api/client"
import { queryKeys } from "@/api/queryKeys"
import { StrategyDetailPage } from "@/routes/strategy-detail-page"
import type {
  StrategyPortfolioNavResponse,
  StrategyPortfolioPerformanceView,
  StrategyPortfolioRecord,
  StrategyPortfolioSignalsResponse,
  StrategyPortfolioSignalTimelineResponse,
  StrategyPortfolioVirtualAccount,
  RuleVersionSpec,
} from "@/types/rearview"

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
  navQuery: {
    data: null as StrategyPortfolioNavResponse | null,
    error: null as unknown,
    isError: false,
    isLoading: false,
  },
  performanceQuery: {
    data: null as StrategyPortfolioPerformanceView | null,
    error: null as unknown,
    isError: false,
    isLoading: false,
  },
  virtualAccountQuery: {
    data: null as StrategyPortfolioVirtualAccount | null,
    error: null as unknown,
    isError: false,
    isLoading: false,
  },
  signalsQuery: {
    data: null as StrategyPortfolioSignalsResponse | null,
    error: null as unknown,
    isError: false,
    isLoading: false,
  },
  signalTimelineQuery: {
    data: null as StrategyPortfolioSignalTimelineResponse | null,
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
  useMetricsQuery: () => ({ ...mocks.emptyQuery, data: [] }),
  useStrategyPortfolioArchiveMutation: () => mocks.archiveMutation,
  useStrategyPortfolioNavQuery: () => mocks.navQuery,
  useStrategyPortfolioPerformanceQuery: () => mocks.performanceQuery,
  useStrategyPortfolioPositionsQuery: () => mocks.emptyQuery,
  useStrategyPortfolioQuery: () => mocks.portfolioQuery,
  useStrategyPortfolioRebalanceRecordsQuery: () => mocks.emptyQuery,
  useStrategyPortfolioSignalsQuery: () => mocks.signalsQuery,
  useStrategyPortfolioSignalTimelineQuery: () => mocks.signalTimelineQuery,
  useStrategyPortfolioStatementQuery: () => mocks.emptyQuery,
  useStrategyPortfolioVirtualAccountQuery: () => mocks.virtualAccountQuery,
}))

vi.mock("lightweight-charts", () => ({
  createChart: () => ({
    addSeries: () => ({ setData: vi.fn() }),
    applyOptions: vi.fn(),
    remove: vi.fn(),
    setCrosshairPosition: vi.fn(),
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
    resetQuery(mocks.navQuery)
    resetQuery(mocks.performanceQuery)
    resetQuery(mocks.virtualAccountQuery)
    resetQuery(mocks.signalsQuery)
    resetQuery(mocks.signalTimelineQuery)
    resetQuery(mocks.emptyQuery)
    mocks.signalsQuery.data = emptySignalsResponse()
    mocks.signalTimelineQuery.data = emptySignalTimelineResponse()
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

  it("opens the strategy config dialog from canonical portfolio config", async () => {
    const record = portfolioRecord()
    mocks.portfolioQuery.data = {
      ...record,
      backtest_segment: {
        ...record.backtest_segment,
        benchmark_security_code: "000905.SH",
      },
      benchmark_security_code: "000905.SH",
      rule_snapshot: strategyConfigRuleSnapshot(),
      ui_display_snapshot: {
        strategy_config_display: {
          version: 1,
          condition_rows: [{ expression: "wrong" }],
          scoring_rows: [],
          build_summary_rows: [],
        },
      },
    }

    await renderPage()

    const configButton = buttonElement("策略配置方案")
    expect(configButton.textContent?.trim()).toBe("策略配置方案")
    expect(configButton.querySelector("svg")).not.toBeNull()
    expect(configButton.className).toContain("font-normal")
    expect(configButton.className).not.toContain("border-border")
    expect(configButton.className).not.toContain("bg-background")
    expect(configButton.getAttribute("title")).toBeNull()
    expect(configButton.hasAttribute("aria-describedby")).toBe(false)

    await clickButton("策略配置方案")

    expect(document.body.textContent).toContain("策略配置")
    expect(
      configDialogElement()?.querySelector("[data-slot='separator']")
    ).not.toBeNull()
    expect(document.body.textContent).not.toContain(
      "近一年 · 2025-06-26 - 2026-06-26 · 中证500（000905.SH） · 建仓日 2026-06-29"
    )
    expect(document.body.textContent).toContain("指标过滤")
    expect(document.body.textContent).toContain("条件组 1")
    expect(document.body.textContent).toContain("close_price > price_ma_20")
    expect(document.body.textContent).not.toContain("wrong")
    expect(document.body.textContent).toContain("权重得分")
    expect(document.body.textContent).toContain("turnover_rate 区间内 2 - 8")
    expect(document.body.textContent).toContain("建仓摘要")
    expect(document.body.textContent).toContain("候选口径")
    expect(document.body.textContent).toContain(
      "Top N 是每日候选信号，不是目标持仓集合"
    )
  })

  it("shows an explicit empty state when canonical config is not parseable", async () => {
    const record = portfolioRecord()
    mocks.portfolioQuery.data = {
      ...record,
      rule_snapshot: {},
      ui_display_snapshot: {
        strategy_config_display: {
          version: 1,
          condition_rows: [],
          scoring_rows: [],
          build_summary_rows: [],
        },
      },
    }

    await renderPage()
    await clickButton("策略配置方案")

    expect(document.body.textContent).toContain("策略配置暂不可展示")
  })

  it("derives the performance benchmark from the source context", async () => {
    const record = portfolioRecord()
    mocks.portfolioQuery.data = {
      ...record,
      backtest_segment: {
        ...record.backtest_segment,
        benchmark_security_code: "000905.SH",
      },
      benchmark_security_code: "000905.SH",
    }

    await renderPage()

    expect(metricValueElement("业绩基准").textContent).toContain("中证500")
    expect(metricValueElement("业绩基准").textContent).toContain("000905.SH")
    expect(metricValueElement("业绩基准").textContent).not.toContain("沪深300")
  })

  it("shows a dashed placeholder when the latest signal date has no buy signals", async () => {
    mocks.signalTimelineQuery.data = {
      ...emptySignalTimelineResponse(),
      trade_dates: [
        {
          signal_count: 0,
          target_count: 0,
          trade_date: "2026-07-02",
        },
      ],
    }

    await renderPage()

    expect(document.body.textContent).toContain("没有产生买入信号")
    expect(document.body.textContent).toContain("2026-07-02")
    expect(document.body.textContent).toContain("0股")
    expect(document.body.textContent).toContain("股票")
    expect(document.body.textContent).toContain("信号 / 建仓")
    expect(document.body.textContent).toContain("得分")
    expect(document.body.textContent).not.toContain("2026-07-02 / 0 只")

    const emptyState = Array.from(
      document.body.querySelectorAll("[data-slot='empty']")
    ).find((candidate) =>
      candidate.textContent?.includes("没有产生买入信号")
    )

    expect(emptyState?.className).toContain("border-dashed")
    const tableFrame = emptyState?.closest("[data-slot='signal-table-frame']")

    expect(tableFrame?.className).toContain("overflow-hidden")
    expect(tableFrame?.className).not.toContain("overflow-y-auto")
    expect(emptyState?.querySelector("[data-slot='empty-title']")).toBeNull()
    expect(
      emptyState?.querySelector("[data-slot='empty-description']")
    ).not.toBeNull()
  })

  it("renders account inception pnl separately from daily pnl", async () => {
    mocks.portfolioQuery.data = livePortfolioRecord()
    mocks.navQuery.data = liveNavResponse()
    mocks.virtualAccountQuery.data = {
      account_date: "2026-06-28",
      cash_balance: 210_000,
      currency: "CNY",
      daily_pnl: -1_200,
      daily_return: -0.0012,
      holding_unrealized_pnl: 5_000,
      position_count: 5,
      position_market_value: 824_500,
      result_attempt_id: "live-attempt-1",
      source: "live_daily_run",
      strategy_portfolio_daily_run_id: "daily-run-1",
      strategy_portfolio_id: "portfolio-1",
      total_equity: 1_034_500,
    }

    await renderPage()

    expect(metricValueElement("总盈亏").textContent).toBe("+34500.00")
    expect(metricValueElement("当日盈亏").textContent).toBe("-1200.00")
  })

  it("renders volatility and alpha metrics with neutral text color", async () => {
    mocks.portfolioQuery.data = livePortfolioRecord()
    mocks.navQuery.data = liveNavResponse()
    mocks.performanceQuery.data = {
      daily_win_rate: {
        observation_count: 2,
        value: 0.5,
        winning_day_count: 1,
      },
      metric: {
        alpha: -0.03,
        annualized_return: 0.08,
        annualized_volatility: 0.22,
        beta: 1.05,
        calmar_ratio: 0.9,
        downside_deviation: 0.12,
        holding_period_return: 0.04,
        information_ratio: 0.2,
        max_drawdown: -0.08,
        sharpe_ratio: 0.7,
        sortino_ratio: 1.1,
        treynor_ratio: 0.5,
      },
      source: "live_daily_run",
      statuses: [],
    }

    await renderPage()

    expect(metricValueElement("年化波动率").className).toContain(
      "text-foreground"
    )
    expect(metricValueElement("下行波动率").className).toContain(
      "text-foreground"
    )
    expect(metricValueElement("Alpha").className).toContain("text-foreground")
    expect(metricValueElement("日胜率").textContent).toBe("50.00%")
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

  function metricValueElement(label: string) {
    const labelElement = Array.from(document.body.querySelectorAll("div")).find(
      (candidate) => candidate.textContent?.trim() === label
    )
    const valueElement = labelElement?.nextElementSibling

    if (!(valueElement instanceof HTMLElement)) {
      throw new Error(`metric value not found: ${label}`)
    }

    return valueElement
  }

  function buttonElement(label: string) {
    const button = Array.from(document.body.querySelectorAll("button")).find(
      (candidate) => candidate.textContent?.includes(label)
    )

    if (!button) {
      throw new Error(`button not found: ${label}`)
    }

    return button
  }

  function configDialogElement() {
    return Array.from(
      document.body.querySelectorAll("[data-slot='dialog-content']")
    ).find((candidate) => candidate.textContent?.includes("策略配置"))
  }

  async function clickButton(label: string) {
    const button = buttonElement(label)

    await act(async () => {
      button.dispatchEvent(new MouseEvent("click", { bubbles: true }))
    })
  }
})

function strategyConfigRuleSnapshot(): RuleVersionSpec {
  return {
    universe: {
      base: "all_a_shares",
      exclude_st: true,
      exclude_suspend: true,
      include_security_codes: [],
      exclude_security_codes: [],
    },
    pool_filters: {
      type: "all",
      conditions: [
        {
          type: "compare",
          left: { type: "metric", name: "close_price" },
          op: "gt",
          right: { type: "metric", name: "price_ma_20" },
        },
      ],
    },
    scoring: {
      rules: [
        {
          type: "conditional_points",
          name: "turnover-band",
          condition: {
            type: "compare",
            left: { type: "metric", name: "turnover_rate" },
            op: "between",
            right: {
              type: "range",
              min: { type: "number", value: 2 },
              max: { type: "number", value: 8 },
            },
          },
          points: 30,
        },
      ],
      clamp: { min: 0, max: 100 },
    },
    top_n_default: 5,
    output_metrics: ["close_price", "price_ma_20", "turnover_rate"],
  }
}

function resetQuery<T>(query: {
  data: T | null
  error: unknown
  isError: boolean
  isLoading: boolean
}) {
  query.data = null
  query.error = null
  query.isError = false
  query.isLoading = false
}

function emptySignalsResponse(): StrategyPortfolioSignalsResponse {
  return {
    has_more: false,
    items: [],
    limit: 50,
    offset: 0,
    pending_buy_signals: [],
    signal_source: "live_daily_run",
    source: "live_daily_run",
  }
}

function emptySignalTimelineResponse(): StrategyPortfolioSignalTimelineResponse {
  return {
    signal_source: "live_daily_run",
    source: "live_daily_run",
    trade_dates: [],
  }
}

function livePortfolioRecord(): StrategyPortfolioRecord {
  const record = portfolioRecord()

  return {
    ...record,
    current_live_result_attempt_id: "live-attempt-1",
    latest_daily_run_id: "daily-run-1",
    live_segment: {
      ...record.live_segment,
      current_live_result_attempt_id: "live-attempt-1",
      latest_daily_run_id: "daily-run-1",
      live_status: "succeeded",
      performance_source: "live_daily_run",
      signal_source: "live_daily_run",
    },
    live_status: "succeeded",
  }
}

function liveNavResponse(): StrategyPortfolioNavResponse {
  return {
    points: [
      {
        benchmark_nav: 1,
        strategy_nav: 1,
        trade_date: "2026-06-27",
      },
      {
        benchmark_nav: 1.01,
        strategy_nav: 1.02,
        trade_date: "2026-06-28",
      },
    ],
    source: "live_daily_run",
  }
}

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
    rule_snapshot: strategyConfigRuleSnapshot(),
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
