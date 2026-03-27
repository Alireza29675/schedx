import { cn } from '../../lib/cn'
import type { ButtonHTMLAttributes } from 'react'

type Variant = 'primary' | 'secondary' | 'danger' | 'ghost'

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: Variant
}

const variants: Record<Variant, string> = {
  primary: 'border-accent text-accent hover:bg-accent-glow hover:shadow-[0_0_12px_rgba(184,224,255,0.2)]',
  secondary: 'border-border bg-bg-panel text-text-main hover:bg-bg-hover hover:border-[#444]',
  danger: 'border-status-failed/50 text-status-failed hover:bg-status-failed/10 hover:border-status-failed',
  ghost: 'border-transparent text-text-muted hover:text-text-main hover:bg-bg-hover',
}

export function Button({ variant = 'secondary', className, children, ...props }: ButtonProps) {
  return (
    <button
      className={cn(
        'border font-mono text-xs uppercase tracking-[0.05em] px-4 py-2 transition-all cursor-pointer',
        'disabled:opacity-40 disabled:cursor-not-allowed',
        variants[variant],
        className,
      )}
      {...props}
    >
      {children}
    </button>
  )
}
