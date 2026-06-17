import type { JobStatus, RunStatus } from '../types'

export const STATUS_COLORS: Record<RunStatus, string> = {
  success: 'text-status-success',
  failed: 'text-status-failed',
  timeout: 'text-status-timeout',
  skipped_overlap: 'text-status-skipped',
  internal_error: 'text-status-error',
}

export const STATUS_BG: Record<RunStatus, string> = {
  success: 'bg-status-success',
  failed: 'bg-status-failed',
  timeout: 'bg-status-timeout',
  skipped_overlap: 'bg-status-skipped',
  internal_error: 'bg-status-error',
}

export const STATUS_LABELS: Record<RunStatus, string> = {
  success: 'Success',
  failed: 'Failed',
  timeout: 'Timeout',
  skipped_overlap: 'Skipped',
  internal_error: 'Error',
}

export const JOB_STATUS_COLORS: Record<JobStatus, string> = {
  active: 'bg-status-active',
  paused: 'bg-status-paused',
  completed: 'bg-status-completed',
  archived: 'bg-status-archived',
}

export const JOB_STATUS_LABELS: Record<JobStatus, string> = {
  active: 'Active',
  paused: 'Paused',
  completed: 'Completed',
  archived: 'Archived',
}

export const ACTION_TYPE_ICONS = {
  command: 'Terminal',
  prompt: 'Robot',
  webhook: 'Globe',
} as const
