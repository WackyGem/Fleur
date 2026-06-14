const ACTIVE_RUN_STATUSES = new Set([
  "created",
  "validating",
  "compiling",
  "running_clickhouse",
  "writing_pool",
  "writing_signals",
])

const TERMINAL_RUN_STATUSES = new Set([
  "succeeded",
  "failed_validation",
  "failed_compile",
  "failed_clickhouse",
  "failed_write",
  "cancelled",
])

const ACTIVE_CHUNK_STATUSES = new Set(["created", "running"])

export function isRunActiveStatus(status?: string | null) {
  return status ? ACTIVE_RUN_STATUSES.has(status) : false
}

export function isRunTerminalStatus(status?: string | null) {
  return status ? TERMINAL_RUN_STATUSES.has(status) : false
}

export function isChunkActiveStatus(status?: string | null) {
  return status ? ACTIVE_CHUNK_STATUSES.has(status) : false
}

export function isFailureStatus(status?: string | null) {
  return status ? status.startsWith("failed") || status === "cancelled" : false
}
