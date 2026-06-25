import type { ComponentProps } from "react"

import { Slider } from "@/components/ui/slider"

type WeightScoreSliderProps = Omit<
  ComponentProps<typeof Slider>,
  "max" | "min" | "onValueChange" | "step" | "value"
> & {
  max?: number
  min?: number
  onValueChange: (value: number) => void
  step?: number
  value: number
}

function WeightScoreSlider({
  max = 100,
  min = 0,
  onValueChange,
  step = 1,
  value,
  ...props
}: WeightScoreSliderProps) {
  return (
    <Slider
      aria-label="权重得分"
      max={max}
      min={min}
      onValueChange={(nextValue) => {
        const score = Array.isArray(nextValue) ? nextValue[0] : nextValue
        if (typeof score === "number") {
          onValueChange(score)
        }
      }}
      step={step}
      value={[value]}
      {...props}
    />
  )
}

export { WeightScoreSlider }
