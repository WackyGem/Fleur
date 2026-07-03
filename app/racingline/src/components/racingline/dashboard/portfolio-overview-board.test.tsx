// @vitest-environment jsdom

import { act } from "react"
import { createRoot, type Root } from "react-dom/client"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

import { PortfolioOverviewBoard } from "@/components/racingline/dashboard/portfolio-overview-board"
import type {
  StrategyPortfolioDashboardCard,
  StrategyPortfolioDashboardResponse,
} from "@/types/rearview"

const mocks = vi.hoisted(() => ({
  dashboardQuery: {
    data: null as StrategyPortfolioDashboardResponse | null,
    error: null as unknown,
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
}))

vi.mock("@/api/hooks", () => ({
  useStrategyPortfolioDashboardQuery: () => mocks.dashboardQuery,
}))

vi.mock("lightweight-charts", () => ({
  createChart: () => ({
    addSeries: () => ({ setData: vi.fn() }),
    applyOptions: vi.fn(),
    remove: vi.fn(),
    timeScale: () => ({ fitContent: vi.fn() }),
  }),
  LineSeries: {},
}))

describe("PortfolioOverviewBoard", () => {
  let container: HTMLDivElement
  let root: Root

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
    mocks.dashboardQuery.data = { portfolios: [] }
    mocks.dashboardQuery.error = null
    mocks.dashboardQuery.isError = false
    mocks.dashboardQuery.isLoading = false
    root = createRoot(container)
  })

  afterEach(() => {
    act(() => {
      root.unmount()
    })
    document.body.innerHTML = ""
    vi.restoreAllMocks()
    vi.useRealTimers()
  })

  it("keeps the initial dashboard load free of flashing loading copy", async () => {
    vi.useFakeTimers()
    mocks.dashboardQuery.data = null
    mocks.dashboardQuery.isLoading = true

    await renderBoard()

    expect(document.body.textContent).not.toContain("策略组合加载中")
    expect(document.body.querySelector("[data-slot='skeleton']")).toBeNull()

    await act(async () => {
      vi.advanceTimersByTime(180)
    })

    expect(document.body.textContent).not.toContain("策略组合加载中")
    expect(document.body.querySelector("[data-slot='skeleton']")).not.toBeNull()
  })

  it("shows a dashed placeholder inside the buy signal table when there are no buy signals", async () => {
    mocks.dashboardQuery.data = {
      portfolios: [dashboardCardWithoutBuySignals()],
    }

    await renderBoard()

    expect(document.body.textContent).toContain("买入信号")
    expect(document.body.textContent).toContain("2026/07/02")
    expect(document.body.textContent).toContain("股票")
    expect(document.body.textContent).toContain("得分")
    expect(document.body.textContent).toContain("没有产生买入信号")

    const emptyState = Array.from(
      document.body.querySelectorAll("[data-slot='empty']")
    ).find((candidate) =>
      candidate.textContent?.includes("没有产生买入信号")
    )

    expect(emptyState?.className).toContain("border-dashed")
    const tableFrame = emptyState?.closest(
      "[data-slot='buy-signal-table-frame']"
    )

    expect(tableFrame?.className).toContain("overflow-hidden")
    expect(tableFrame?.className).not.toContain("overflow-y-auto")
    expect(emptyState?.querySelector("[data-slot='empty-title']")).toBeNull()
    expect(
      emptyState?.querySelector("[data-slot='empty-description']")
    ).not.toBeNull()
  })

  async function renderBoard() {
    await act(async () => {
      root.render(<PortfolioOverviewBoard />)
    })
  }
})

function dashboardCardWithoutBuySignals(): StrategyPortfolioDashboardCard {
  return {
    backtest_segment: {
      benchmark_security_code: "000300.SH",
      end_date: "2026-06-26",
      period_key: "1y",
      source_result_attempt_id: "attempt-1",
      source_strategy_backtest_run_id: "backtest-1",
      start_date: "2025-06-26",
    },
    created_at: "2026-06-27T00:00:00Z",
    current_result_attempt_id: "attempt-1",
    curve: [{ benchmark: 1.01, nav: 1.02, time: "2026-07-02" }],
    curve_source: "live_daily_run",
    efficiency: [],
    initial_signal_date: "2026-06-26",
    latest_daily_run_id: "daily-run-1",
    latest_nav: 1.02,
    live_segment: {
      current_live_result_attempt_id: "attempt-1",
      initial_signal_date: "2026-06-26",
      latest_daily_run_id: "daily-run-1",
      live_start_date: "2026-06-29",
      live_status: "succeeded",
      performance_source: "live_daily_run",
      signal_source: "live_daily_run",
    },
    live_status: "succeeded",
    live_summary: null,
    live_start_date: "2026-06-29",
    name: "低位反转组合",
    pending_buy_signals: [],
    portfolio_code: "PF-20260627-0001",
    recent_change: 0,
    relative: [],
    returns: [],
    risk: [],
    source_backtest_summary: {},
    source_end_date: "2026-06-26",
    source_period_key: "1y",
    source_result_attempt_id: "attempt-1",
    source_start_date: "2025-06-26",
    source_strategy_backtest_run_id: "backtest-1",
    status: "active",
    strategy_portfolio_id: "portfolio-1",
    today_signals: [],
    ui_display_snapshot: {},
    updated_at: "2026-07-02T00:00:00Z",
  }
}
