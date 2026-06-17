import type { RunRecord } from '../types'
import { api } from './client'

export const runsApi = {
  get: (runId: string) => api.get<RunRecord>(`/api/runs/${runId}`),

  getLogs: (runId: string) => api.get<string>(`/api/runs/${runId}/logs`),
}
