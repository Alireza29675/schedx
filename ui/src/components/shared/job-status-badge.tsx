import { Badge } from '../ui/badge'
import { JOB_STATUS_COLORS, JOB_STATUS_LABELS } from '../../lib/constants'
import type { JobStatus } from '../../types'

interface JobStatusBadgeProps {
  status: JobStatus
  className?: string
}

export function JobStatusBadge({ status, className }: JobStatusBadgeProps) {
  return (
    <Badge
      color={JOB_STATUS_COLORS[status]}
      glow={status === 'active'}
      className={className}
    >
      {JOB_STATUS_LABELS[status]}
    </Badge>
  )
}
