export type JobStatus = 'active' | 'paused' | 'completed' | 'archived'

export type ActionType = 'command' | 'prompt' | 'webhook'

export type HttpMethod = 'GET' | 'POST' | 'PUT' | 'PATCH' | 'DELETE'

export interface CommandAction {
  type: 'command'
  command: string
  shell: boolean
  workdir?: string
}

export interface PromptAction {
  type: 'prompt'
  text: string
  agent?: string
}

export interface WebhookAction {
  type: 'webhook'
  url: string
  method: HttpMethod
  headers: [string, string][]
  body?: string
}

export type Action = CommandAction | PromptAction | WebhookAction

export interface LastRun {
  run_id: string
  status: RunStatus
  started_at: string
  finished_at: string
  exit_code?: number
}

export type RunStatus = 'success' | 'failed' | 'timeout' | 'skipped_overlap' | 'internal_error'

export interface Job {
  id: string
  name?: string
  status: JobStatus
  schedule_input: string
  schedule: JobSchedule
  action: Action
  timeout_seconds: number
  tags: string[]
  created_at: string
  updated_at: string
  last_scheduled_at?: string
  last_run?: LastRun
  run_count: number
  skip_remaining: number
  in_flight?: InFlightRun
}

export interface InFlightRun {
  run_id: string
  scheduled_for: string
  claimed_at: string
}

export type JobSchedule =
  | { kind: 'recurring_cron'; expr: string }
  | { kind: 'recurring_interval'; every_seconds: number }
  | { kind: 'one_shot'; fire_at: string }

export interface JobListEntry {
  id: string
  name?: string
  status: JobStatus
  action_type: ActionType
  schedule_input: string
  tags: string[]
  last_run?: LastRun
  next_run?: string
  run_count: number
  created_at: string
  in_flight?: InFlightRun
}
