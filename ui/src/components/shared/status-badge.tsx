import { Badge } from '../ui/badge'
import { STATUS_BG, STATUS_LABELS } from '../../lib/constants'
import type { RunStatus } from '../../types'

interface StatusBadgeProps {
  status: RunStatus
  className?: string
}

export function StatusBadge({ status, className }: StatusBadgeProps) {
  return (
    <Badge color={STATUS_BG[status]} className={className}>
      {STATUS_LABELS[status]}
    </Badge>
  )
}
