import { describe, expect, it } from "vitest"

import {
  isChunkActiveStatus,
  isFailureStatus,
  isRunActiveStatus,
  isRunTerminalStatus,
} from "@/lib/status"

describe("run and chunk status helpers", () => {
  it("classifies active run statuses", () => {
    expect(isRunActiveStatus("created")).toBe(true)
    expect(isRunActiveStatus("running_clickhouse")).toBe(true)
    expect(isRunActiveStatus("succeeded")).toBe(false)
    expect(isRunActiveStatus(null)).toBe(false)
  })

  it("classifies terminal run statuses", () => {
    expect(isRunTerminalStatus("succeeded")).toBe(true)
    expect(isRunTerminalStatus("failed_compile")).toBe(true)
    expect(isRunTerminalStatus("writing_pool")).toBe(false)
  })

  it("classifies active chunk and failure statuses", () => {
    expect(isChunkActiveStatus("running")).toBe(true)
    expect(isChunkActiveStatus("succeeded")).toBe(false)
    expect(isFailureStatus("failed_write")).toBe(true)
    expect(isFailureStatus("cancelled")).toBe(true)
  })
})
