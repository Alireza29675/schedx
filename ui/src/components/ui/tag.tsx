import { cn } from '../../lib/cn'

interface TagProps {
  children: React.ReactNode
  className?: string
}

export function Tag({ children, className }: TagProps) {
  return (
    <span className={cn(
      'border border-border text-[10px] font-mono px-2 py-0.5 rounded-sm text-text-muted',
      className,
    )}>
      {children}
    </span>
  )
}
