import { HugeiconsIcon } from "@hugeicons/react"
import type { HugeiconsIconProps } from "@hugeicons/react"

type RacinglineIconProps = HugeiconsIconProps & {
  inline?: "start" | "end"
}

export function RacinglineIcon({
  inline,
  strokeWidth = 2,
  ...props
}: RacinglineIconProps) {
  return (
    <HugeiconsIcon
      data-icon={
        inline === "start"
          ? "inline-start"
          : inline === "end"
            ? "inline-end"
            : undefined
      }
      strokeWidth={strokeWidth}
      {...props}
    />
  )
}
