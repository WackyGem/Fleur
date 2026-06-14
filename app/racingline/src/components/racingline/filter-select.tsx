import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"

export type SelectOption = {
  label: string
  value: string | null
}

type FilterSelectProps = {
  options: SelectOption[]
  value: string
  onValueChange: (value: string) => void
  className?: string
  size?: "sm" | "default"
}

export function FilterSelect({
  options,
  value,
  onValueChange,
  className,
  size = "sm",
}: FilterSelectProps) {
  return (
    <Select
      items={options}
      value={value || null}
      onValueChange={(nextValue) => onValueChange(String(nextValue ?? ""))}
    >
      <SelectTrigger className={className} size={size}>
        <SelectValue />
      </SelectTrigger>
      <SelectContent alignItemWithTrigger={false}>
        <SelectGroup>
          {options.map((option) => (
            <SelectItem key={option.value ?? "all"} value={option.value}>
              {option.label}
            </SelectItem>
          ))}
        </SelectGroup>
      </SelectContent>
    </Select>
  )
}
