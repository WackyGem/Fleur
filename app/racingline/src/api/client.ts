import type { ListResult } from "@/types/rearview"

const DEFAULT_LIMIT = 50

type ApiErrorBody = {
  error_type?: string
  message?: string
  field_path?: string
}

export class ApiError extends Error {
  readonly status: number
  readonly errorType?: string
  readonly fieldPath?: string

  constructor(status: number, body: ApiErrorBody, fallbackMessage: string) {
    super(body.message || fallbackMessage)
    this.name = "ApiError"
    this.status = status
    this.errorType = body.error_type
    this.fieldPath = body.field_path
  }
}

export type QueryValue =
  | string
  | number
  | boolean
  | null
  | undefined

export type QueryParams = Record<string, QueryValue>

export function apiBaseUrl() {
  const configured = import.meta.env.VITE_REARVIEW_API_BASE_URL
  return configured?.replace(/\/+$/, "") || "http://127.0.0.1:34057"
}

export function buildPath(path: string, query?: QueryParams) {
  const params = new URLSearchParams()
  for (const [key, value] of Object.entries(query ?? {})) {
    if (value === undefined || value === null || value === "") {
      continue
    }
    params.set(key, String(value))
  }
  const queryString = params.toString()
  return `${path}${queryString ? `?${queryString}` : ""}`
}

export async function requestJson<T>(
  path: string,
  options: RequestInit = {},
): Promise<T> {
  const response = await fetch(`${apiBaseUrl()}${path}`, {
    ...options,
    headers: {
      Accept: "application/json",
      ...(options.body ? { "Content-Type": "application/json" } : {}),
      ...options.headers,
    },
  })

  if (!response.ok) {
    throw await toApiError(response)
  }

  if (response.status === 204) {
    return undefined as T
  }

  return (await response.json()) as T
}

export function jsonBody<T>(body: T): RequestInit {
  return {
    body: JSON.stringify(body),
    method: "POST",
  }
}

export function normalizeList<T>(
  value: T[] | Partial<ListResult<T>> | { data?: T[] },
  fallbackLimit = DEFAULT_LIMIT,
): ListResult<T> {
  if (Array.isArray(value)) {
    return {
      items: value,
      limit: fallbackLimit,
      offset: 0,
      has_more: false,
    }
  }

  const objectValue = value as Partial<ListResult<T>> & { data?: T[] }
  const items = Array.isArray(objectValue.items)
    ? objectValue.items
    : Array.isArray(objectValue.data)
      ? objectValue.data
      : []

  return {
    items,
    limit: objectValue.limit ?? fallbackLimit,
    offset: objectValue.offset ?? 0,
    has_more: objectValue.has_more ?? false,
    total: objectValue.total,
  }
}

async function toApiError(response: Response) {
  try {
    const body = (await response.json()) as ApiErrorBody
    return new ApiError(
      response.status,
      body,
      `Rearview request failed with HTTP ${response.status}`,
    )
  } catch {
    return new ApiError(
      response.status,
      {},
      `Rearview request failed with HTTP ${response.status}`,
    )
  }
}
