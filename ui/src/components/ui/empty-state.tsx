interface EmptyStateProps {
  icon?: React.ReactNode
  title: string
  description?: string
  action?: React.ReactNode
}

export function EmptyState({ icon, title, description, action }: EmptyStateProps) {
  return (
    <div className="flex flex-col items-center justify-center py-16 gap-4">
      {icon && <div className="text-text-muted text-4xl">{icon}</div>}
      <div className="text-center">
        <h3 className="font-heading text-lg text-text-main">{title}</h3>
        {description && <p className="text-sm text-text-muted mt-1">{description}</p>}
      </div>
      {action}
    </div>
  )
}
