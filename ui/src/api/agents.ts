import type { AgentProfile } from '../types'
import { api } from './client'

export const agentsApi = {
  list: () => api.get<AgentProfile[]>('/api/agents'),

  create: (body: { name: string; bin: string; args: string[]; prompt_stdin: boolean }) =>
    api.post<AgentProfile>('/api/agents', body),

  update: (name: string, body: Record<string, unknown>) =>
    api.patch<AgentProfile>(`/api/agents/${name}`, body),

  remove: (name: string) => api.delete<void>(`/api/agents/${name}`),

  setDefault: (name: string) => api.post<void>(`/api/agents/${name}/default`),
}
