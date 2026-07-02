import { ApiError } from "@/api/client"

export function isArchivedPortfolioError(error: unknown) {
  return error instanceof ApiError && error.status === 410
}

export function strategyPortfolioArchiveErrorMessage(error: unknown) {
  if (error instanceof ApiError && error.message.trim()) {
    return error.message
  }

  return "删除失败，请稍后重试。"
}
