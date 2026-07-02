import { describe, expect, it } from "vitest"

import { ApiError } from "@/api/client"
import {
  isArchivedPortfolioError,
  strategyPortfolioArchiveErrorMessage,
} from "@/routes/strategy-detail-utils"

describe("strategy detail utils", () => {
  it("recognizes HTTP 410 as archived portfolio error", () => {
    const error = new ApiError(410, { error_type: "gone" }, "gone")

    expect(isArchivedPortfolioError(error)).toBe(true)
  })

  it("does not treat HTTP 404 as archived portfolio error", () => {
    const error = new ApiError(404, { error_type: "not_found" }, "not found")

    expect(isArchivedPortfolioError(error)).toBe(false)
  })

  it("uses backend message for archive failures", () => {
    const error = new ApiError(
      500,
      { message: "Rearview archive failed" },
      "fallback"
    )

    expect(strategyPortfolioArchiveErrorMessage(error)).toBe(
      "Rearview archive failed"
    )
  })

  it("uses fallback message for unknown archive failures", () => {
    expect(strategyPortfolioArchiveErrorMessage(new Error("network"))).toBe(
      "删除失败，请稍后重试。"
    )
  })
})
