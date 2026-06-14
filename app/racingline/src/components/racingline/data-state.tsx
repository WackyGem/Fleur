import type { ReactNode } from "react"

import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert"
import { Button } from "@/components/ui/button"
import {
  Empty,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from "@/components/ui/empty"
import { Skeleton } from "@/components/ui/skeleton"
import { describeError } from "@/lib/format"
import { RacinglineIcon } from "@/components/racingline/icon"
import {
  AlertCircleIcon,
  DatabaseSearchIcon,
} from "@hugeicons/core-free-icons"

type ErrorStateProps = {
  title: string
  error: unknown
  action?: ReactNode
}

export function ErrorState({ title, error, action }: ErrorStateProps) {
  return (
    <Alert variant="destructive">
      <RacinglineIcon icon={AlertCircleIcon} />
      <AlertTitle>{title}</AlertTitle>
      <AlertDescription className="flex flex-col gap-2">
        <span>{describeError(error)}</span>
        {action}
      </AlertDescription>
    </Alert>
  )
}

type MissingBackendStateProps = {
  title: string
  description: string
  retry?: () => void
}

export function MissingBackendState({
  title,
  description,
  retry,
}: MissingBackendStateProps) {
  return (
    <Empty>
      <EmptyHeader>
        <EmptyMedia variant="icon">
          <RacinglineIcon icon={DatabaseSearchIcon} />
        </EmptyMedia>
        <EmptyTitle>{title}</EmptyTitle>
        <EmptyDescription>{description}</EmptyDescription>
      </EmptyHeader>
      {retry ? (
        <Button variant="outline" size="sm" onClick={retry}>
          Retry
        </Button>
      ) : null}
    </Empty>
  )
}

export function TableSkeleton({ rows = 5 }: { rows?: number }) {
  return (
    <div className="flex flex-col gap-2">
      {Array.from({ length: rows }, (_, index) => (
        <Skeleton key={index} className="h-9 w-full" />
      ))}
    </div>
  )
}
