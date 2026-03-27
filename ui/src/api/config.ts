import type { AppConfig, SystemStatus } from '../types'
import { api } from './client'

export const configApi = {
  get: () => api.get<AppConfig>('/api/config'),

  update: (body: Partial<AppConfig>) => api.patch<AppConfig>('/api/config', body),

  status: () => api.get<SystemStatus>('/api/status'),

  repair: () => api.post<{ message: string }>('/api/repair', { confirm: true }),

  validateSchedule: (schedule: string) =>
    api.post<{ valid: boolean; description?: string; error?: string }>('/api/validate/schedule', { schedule }),
}
