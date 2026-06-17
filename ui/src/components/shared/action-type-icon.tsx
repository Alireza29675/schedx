import { Terminal, Robot, Globe } from '@phosphor-icons/react'
import type { ActionType } from '../../types'

const icons = {
  command: Terminal,
  prompt: Robot,
  webhook: Globe,
}

const labels: Record<ActionType, string> = {
  command: 'Command',
  prompt: 'Prompt',
  webhook: 'Webhook',
}

interface ActionTypeIconProps {
  type: ActionType
  showLabel?: boolean
  className?: string
}

export function ActionTypeIcon({ type, showLabel = true, className }: ActionTypeIconProps) {
  const Icon = icons[type]
  return (
    <div className={`flex items-center gap-2 ${className ?? ''}`}>
      <Icon className="text-text-muted" size={16} />
      {showLabel && <span className="font-mono text-xs">{labels[type]}</span>}
    </div>
  )
}
