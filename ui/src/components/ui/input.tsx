import { cn } from '../../lib/cn'
import { forwardRef } from 'react'
import type { InputHTMLAttributes } from 'react'

interface InputProps extends InputHTMLAttributes<HTMLInputElement> {
  label?: string
}

export const Input = forwardRef<HTMLInputElement, InputProps>(
  ({ label, className, ...props }, ref) => (
    <div className="flex flex-col gap-1.5">
      {label && <label className="label-mono">{label}</label>}
      <input
        ref={ref}
        className={cn(
          'bg-bg-input border border-border text-sm font-mono text-text-main py-2 px-3 placeholder:text-text-muted',
          className,
        )}
        {...props}
      />
    </div>
  ),
)

Input.displayName = 'Input'
