export interface AgentProfile {
  name: string
  bin: string
  args: string[]
  prompt_stdin: boolean
  is_default: boolean
  job_count: number
}
