import { describe, expect, it } from "vitest"

import { queryKeys } from "@/api/queryKeys"

describe("strategy backtest query keys", () => {
  it("keeps full run and status views in separate caches", () => {
    expect(queryKeys.strategyBacktest("run-1")).not.toEqual(
      queryKeys.strategyBacktestStatus("run-1")
    )
  })

  it("keeps full and ui result wrapper views in separate caches", () => {
    expect(queryKeys.strategyBacktestOverviewUi("run-1", "2025-01-02")).not.toEqual(
      queryKeys.strategyBacktestNavUi("run-1")
    )
    expect(queryKeys.strategyBacktestOverviewUi("run-1", "2025-01-02")).not.toEqual(
      queryKeys.strategyBacktestPerformanceUi("run-1")
    )
    expect(queryKeys.strategyBacktestNav("run-1")).not.toEqual(
      queryKeys.strategyBacktestNavUi("run-1")
    )
    expect(queryKeys.strategyBacktestPerformance("run-1")).not.toEqual(
      queryKeys.strategyBacktestPerformanceUi("run-1")
    )
    expect(queryKeys.strategyBacktestRebalanceRecords("run-1", "2025-01-02")).not.toEqual(
      queryKeys.strategyBacktestRebalanceRecordsUi("run-1", "2025-01-02")
    )
  })
})
