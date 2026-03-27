import type { RunRecord } from '../types'
import { api } from './client'

export interface ListHistoryParams {
  job_id?: string
  status?: string
  trigger?: string
  from?: string
  to?: string
  limit?: number
  offset?: number
  sort?: string
  order?: 'asc' | 'desc'
}

function buildQuery(params: ListHistoryParams): string {
  const sp = new URLSearchParams()
  for (const [k, v] of Object.entries(params)) {
    if (v !== undefined && v !== '') sp.set(k, String(v))
  }
  const qs = sp.toString()
  return qs ? `?${qs}` : ''
}

export const historyApi = {
  list: (params: ListHistoryParams = {}) =>
    api.get<{ records: RunRecord[]; total: number }>(`/api/history${buildQuery(params)}`),

  listForJob: (jobId: string, params: Omit<ListHistoryParams, 'job_id'> = {}) =>
    api.get<{ records: RunRecord[]; total: number }>(`/api/jobs/${jobId}/history${buildQuery(params)}`),
}
