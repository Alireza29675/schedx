import type { RunStatus } from './job'

export type Trigger = 'scheduled' | 'manual'

export interface RunRecord {
  run_id: string
  job_id: string
  job_name?: string
  trigger: Trigger
  scheduled_for: string
  started_at: string
  finished_at: string
  status: RunStatus
  exit_code?: number
  log_path: string
}
