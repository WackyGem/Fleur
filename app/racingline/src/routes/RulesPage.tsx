import { RuleWorkbench } from "@/features/rules/components/rule-workbench"

export function RulesPage() {
  return (
    <div className="flex flex-col gap-4">
      <div>
        <h1 className="text-xl font-medium">Rules</h1>
        <p className="text-sm text-muted-foreground">
          Rule sets, immutable versions, explain, publish and run launch.
        </p>
      </div>
      <RuleWorkbench />
    </div>
  )
}
