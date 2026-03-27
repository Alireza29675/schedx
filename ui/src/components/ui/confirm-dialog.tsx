import { Button } from './button'

interface ConfirmDialogProps {
  open: boolean
  title: string
  description: string
  confirmLabel?: string
  variant?: 'danger' | 'primary'
  onConfirm: () => void
  onCancel: () => void
}

export function ConfirmDialog({
  open,
  title,
  description,
  confirmLabel = 'Confirm',
  variant = 'danger',
  onConfirm,
  onCancel,
}: ConfirmDialogProps) {
  if (!open) return null

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      <div className="absolute inset-0 bg-black/60 backdrop-blur-sm" onClick={onCancel} />
      <div className="relative bg-bg-panel border border-border p-6 max-w-md w-full mx-4 shadow-2xl">
        <h3 className="font-heading text-lg text-text-main">{title}</h3>
        <p className="text-sm text-text-muted mt-2">{description}</p>
        <div className="flex justify-end gap-3 mt-6">
          <Button variant="secondary" onClick={onCancel}>Cancel</Button>
          <Button variant={variant} onClick={onConfirm}>{confirmLabel}</Button>
        </div>
      </div>
    </div>
  )
}
