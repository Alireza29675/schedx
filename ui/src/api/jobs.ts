import type { Job, JobListEntry } from '../types'
import { api } from './client'

export interface ListJobsParams {
  status?: string
  tag?: string
  action_type?: string
  search?: string
  sort?: string
  order?: 'asc' | 'desc'
}

function buildQuery(params: ListJobsParams): string {
  const sp = new URLSearchParams()
  for (const [k, v] of Object.entries(params)) {
    if (v) sp.set(k, v)
  }
  const qs = sp.toString()
  return qs ? `?${qs}` : ''
}

export const jobsApi = {
  list: (params: ListJobsParams = {}) =>
    api.get<JobListEntry[]>(`/api/jobs${buildQuery(params)}`),

  get: (id: string) => api.get<Job>(`/api/jobs/${id}`),

  create: (body: Record<string, unknown>) =>
    api.post<Job>('/api/jobs', body),

  update: (id: string, body: Record<string, unknown>) =>
    api.patch<Job>(`/api/jobs/${id}`, body),

  remove: (id: string) => api.delete<void>(`/api/jobs/${id}`),

  run: (id: string) => api.post<void>(`/api/jobs/${id}/run`),

  pause: (id: string) => api.post<void>(`/api/jobs/${id}/pause`),

  resume: (id: string) => api.post<void>(`/api/jobs/${id}/resume`),

  skip: (id: string, times: number) =>
    api.post<void>(`/api/jobs/${id}/skip`, { times }),

  unarchive: (id: string) => api.post<void>(`/api/jobs/${id}/unarchive`),
}
