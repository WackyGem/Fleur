import { SheetDescription } from "@/components/ui/sheet"
import {
  Sheet,
  SheetContent,
  SheetHeader,
  SheetTitle,
} from "@/components/ui/sheet"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Badge } from "@/components/ui/badge"
import { formatScore, jsonEntries, displayJsonValue } from "@/lib/format"
import type { BuySignalRecord, JsonValue } from "@/types/rearview"

type SignalDetailSheetProps = {
  signal: BuySignalRecord | null
  open: boolean
  onOpenChange: (open: boolean) => void
}

export function SignalDetailSheet({
  signal,
  open,
  onOpenChange,
}: SignalDetailSheetProps) {
  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent className="sm:max-w-xl">
        <SheetHeader>
          <SheetTitle>{signal?.security_code ?? "Signal detail"}</SheetTitle>
          <SheetDescription>
            PostgreSQL run snapshot for {signal?.trade_date ?? "-"}.
          </SheetDescription>
        </SheetHeader>
        {signal ? (
          <div className="flex flex-col gap-3 overflow-auto px-4 pb-4">
            <div className="grid gap-2 sm:grid-cols-3">
              <Stat label="Rank" value={String(signal.rank)} />
              <Stat label="Score" value={formatScore(signal.score)} />
              <Stat label="Run date" value={signal.trade_date} />
            </div>
            <JsonCard
              entries={jsonEntries(signal.score_breakdown)}
              title="score_breakdown"
            />
            <JsonCard
              entries={jsonEntries(signal.selected_metrics)}
              title="selected_metrics"
            />
          </div>
        ) : null}
      </SheetContent>
    </Sheet>
  )
}

function Stat({ label, value }: { label: string; value: string }) {
  return (
    <Card size="sm">
      <CardHeader>
        <CardTitle>{label}</CardTitle>
      </CardHeader>
      <CardContent>
        <Badge variant="secondary">{value}</Badge>
      </CardContent>
    </Card>
  )
}

function JsonCard({
  entries,
  title,
}: {
  entries: Array<[string, JsonValue]>
  title: string
}) {
  return (
    <Card size="sm">
      <CardHeader>
        <CardTitle>{title}</CardTitle>
      </CardHeader>
      <CardContent>
        <div className="grid gap-2">
          {entries.length === 0 ? (
            <div className="text-sm text-muted-foreground">-</div>
          ) : (
            entries.map(([key, value]) => (
              <div
                key={key}
                className="grid gap-1 rounded-lg bg-muted p-2 sm:grid-cols-[minmax(8rem,12rem)_1fr]"
              >
                <div className="truncate font-mono text-xs">{key}</div>
                <div className="min-w-0 break-words text-xs">
                  {displayJsonValue(value)}
                </div>
              </div>
            ))
          )}
        </div>
      </CardContent>
    </Card>
  )
}
