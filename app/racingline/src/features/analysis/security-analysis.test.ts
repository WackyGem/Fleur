import { describe, expect, it } from "vitest"

import {
  buildRunDetailPath,
  buildSecurityAnalysisPath,
  buildSecurityAnalysisQuery,
  nextMaWindows,
  parseAnalysisSource,
  parsePriceAdjustment,
  quoteForTradeDate,
} from "@/features/analysis/security-analysis"
import type { QuoteMartRow } from "@/types/rearview"

describe("security analysis helpers", () => {
  it("keeps only supported source and adjustment values", () => {
    expect(parseAnalysisSource("signals")).toBe("signals")
    expect(parseAnalysisSource("pool")).toBe("pool")
    expect(parseAnalysisSource("unknown")).toBeNull()
    expect(parsePriceAdjustment("backward_adjusted")).toBe("backward_adjusted")
    expect(parsePriceAdjustment("bad-adjustment")).toBe("forward_adjusted")
    expect(parsePriceAdjustment(null)).toBe("forward_adjusted")
  })

  it("builds shareable analysis and return paths", () => {
    expect(
      buildSecurityAnalysisPath({
        runId: "run-1",
        securityCode: "000001.SZ",
        source: "signals",
        tradeDate: "2026-06-12",
      }),
    ).toBe(
      "/runs/run-1/securities/000001.SZ?adjustment=forward_adjusted&source=signals&trade_date=2026-06-12",
    )
    expect(
      buildRunDetailPath({
        runId: "run-1",
        source: "pool",
        tradeDate: "2026-06-12",
      }),
    ).toBe("/runs/run-1?source=pool&trade_date=2026-06-12")
  })

  it("only builds API query when source and trade date are complete", () => {
    expect(
      buildSecurityAnalysisQuery({
        adjustment: "forward_adjusted",
        source: "signals",
        tradeDate: "2026-06-12",
      }),
    ).toEqual({
      adjustment: "forward_adjusted",
      source: "signals",
      trade_date: "2026-06-12",
    })
    expect(
      buildSecurityAnalysisQuery({
        adjustment: "forward_adjusted",
        source: null,
        tradeDate: "2026-06-12",
      }),
    ).toBeUndefined()
  })

  it("keeps MA window toggles inside the supported set", () => {
    expect(nextMaWindows([5, 10, 30], 10, false)).toEqual([5, 30])
    expect(nextMaWindows([5], 30, true)).toEqual([5, 30])
    expect(nextMaWindows([5, 10], 60, true)).toEqual([5, 10])
  })

  it("selects a quote row by selected day without falling back", () => {
    const rows: QuoteMartRow[] = [
      { security_code: "000001.SZ", trade_date: "2026-06-11" },
      { security_code: "000001.SZ", trade_date: "2026-06-12" },
    ]
    expect(quoteForTradeDate(rows, "2026-06-12")).toEqual(rows[1])
    expect(quoteForTradeDate(rows, "2026-06-13")).toBeNull()
  })
})
