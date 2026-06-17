import { cn } from '../../lib/cn'

interface BadgeProps {
  color: string
  glow?: boolean
  children: React.ReactNode
  className?: string
}

export function Badge({ color, glow, children, className }: BadgeProps) {
  return (
    <div className={cn('flex items-center gap-2', className)}>
      <div
        className={cn('w-2 h-2 rounded-full', color)}
        style={glow ? { boxShadow: `0 0 5px currentColor` } : undefined}
      />
      <span className="font-mono text-xs text-text-muted">{children}</span>
    </div>
  )
}
