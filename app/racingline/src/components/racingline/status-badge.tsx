import { Badge } from "@/components/ui/badge"
import { isFailureStatus, isRunActiveStatus } from "@/lib/status"

type StatusBadgeProps = {
  status?: string | null
}

export function StatusBadge({ status }: StatusBadgeProps) {
  if (!status) {
    return <Badge variant="outline">unknown</Badge>
  }

  if (isFailureStatus(status)) {
    return <Badge variant="destructive">{status}</Badge>
  }

  if (isRunActiveStatus(status) || status === "running") {
    return <Badge>{status}</Badge>
  }

  if (status === "succeeded") {
    return <Badge variant="secondary">{status}</Badge>
  }

  return <Badge variant="outline">{status}</Badge>
}
