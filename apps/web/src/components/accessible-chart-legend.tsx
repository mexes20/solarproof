import type { LegendProps } from 'recharts'

interface LegendItem {
  value?: string
  color?: string
  description: string
}

export function AccessibleLegend({
  payload,
  items,
  className = 'flex justify-center gap-4 pt-2.5 text-[11px] text-gray-600 dark:text-gray-400',
}: LegendProps & { items: LegendItem[]; className?: string }) {
  if (!payload?.length) return null

  const descriptions = new Map(items.map((item) => [item.value, item.description]))

  return (
    <ul className={className} aria-label="Chart legend">
      {payload.map((entry) => {
        const label = String(entry.value ?? '')
        const description = descriptions.get(label) ?? label
        return (
          <li key={label} className="flex items-center gap-1.5">
            <span
              className="inline-block h-2 w-2 rounded-full"
              style={{ backgroundColor: entry.color }}
              aria-hidden="true"
            />
            <span aria-label={description}>{label}</span>
          </li>
        )
      })}
    </ul>
  )
}
