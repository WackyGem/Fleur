import { Plus } from "lucide-react"

import { cn } from "@/lib/utils"

type AddDashedButtonProps = {
  className?: string
  description?: string
  label: string
  onClick: () => void
  size?: "compact" | "large"
}

function AddDashedButton({
  className,
  description,
  label,
  onClick,
  size = "compact",
}: AddDashedButtonProps) {
  const large = size === "large"

  return (
    <button
      className={cn(
        "group w-full border border-dashed border-border/80 bg-muted/10 text-center transition-colors hover:border-foreground/50 hover:bg-muted/25 focus-visible:ring-1 focus-visible:ring-ring focus-visible:outline-none",
        large
          ? "flex min-h-32 flex-col items-center justify-center gap-2 px-6 py-6"
          : "flex min-h-14 items-center justify-center gap-2 px-4 py-3 text-xs font-medium text-foreground",
        className
      )}
      onClick={onClick}
      type="button"
    >
      <span className="flex items-center gap-2">
        <span
          className={cn(
            "flex items-center justify-center border border-dashed border-border/90 text-muted-foreground transition-colors group-hover:border-foreground/50 group-hover:text-foreground",
            large ? "size-9" : "size-7"
          )}
        >
          <Plus />
        </span>
        <span className={cn(large && "text-sm font-medium text-foreground")}>
          {label}
        </span>
      </span>
      {description ? (
        <span className="max-w-md text-xs leading-relaxed text-muted-foreground">
          {description}
        </span>
      ) : null}
    </button>
  )
}

export { AddDashedButton }
