import { useToast } from '../../hooks/use-toast'
import { cn } from '../../lib/cn'
import { X } from '@phosphor-icons/react'

const typeStyles = {
  success: 'border-status-success/50 text-status-success',
  error: 'border-status-failed/50 text-status-failed',
  info: 'border-accent/50 text-accent',
}

export function ToastContainer() {
  const { toasts, removeToast } = useToast()

  return (
    <div className="fixed bottom-6 right-6 z-50 flex flex-col gap-2">
      {toasts.map((toast) => (
        <div
          key={toast.id}
          className={cn(
            'bg-bg-panel border px-4 py-3 flex items-center gap-3 shadow-xl min-w-[280px] animate-in slide-in-from-right',
            typeStyles[toast.type],
          )}
        >
          <span className="font-mono text-xs flex-1">{toast.message}</span>
          <button
            onClick={() => removeToast(toast.id)}
            className="text-text-muted hover:text-text-main transition-colors"
          >
            <X size={14} />
          </button>
        </div>
      ))}
    </div>
  )
}
