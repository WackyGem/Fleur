import { describe, expect, it } from "vitest"

import { ApiError, buildPath, normalizeList } from "@/api/client"

describe("API client helpers", () => {
  it("serializes query params and drops empty values", () => {
    expect(
      buildPath("/rearview/runs", {
        keyword: "",
        limit: 50,
        offset: 0,
        status: "succeeded",
        unset: undefined,
      }),
    ).toBe("/rearview/runs?limit=50&offset=0&status=succeeded")
  })

  it("normalizes array list responses", () => {
    expect(normalizeList([{ run_id: "r1" }], 25)).toEqual({
      has_more: false,
      items: [{ run_id: "r1" }],
      limit: 25,
      offset: 0,
    })
  })

  it("keeps paged response metadata", () => {
    expect(
      normalizeList({
        has_more: true,
        items: ["a"],
        limit: 1,
        offset: 2,
        total: 5,
      }),
    ).toEqual({
      has_more: true,
      items: ["a"],
      limit: 1,
      offset: 2,
      total: 5,
    })
  })

  it("maps structured API errors", () => {
    const error = new ApiError(
      400,
      {
        error_type: "validation_error",
        field_path: "rule.pool_filters",
        message: "invalid rule",
      },
      "fallback",
    )

    expect(error.message).toBe("invalid rule")
    expect(error.status).toBe(400)
    expect(error.errorType).toBe("validation_error")
    expect(error.fieldPath).toBe("rule.pool_filters")
  })
})
