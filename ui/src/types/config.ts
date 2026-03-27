export interface AppConfig {
  backend: string
  backend_status: string
  allow_insecure_http: boolean
  default_timeout_seconds: number
  backup_count: number
  log_retention_days: number
  archive_after_hours: number
  default_agent?: string
}

export interface SystemStatus {
  version: string
  schedx_home: string
  os: string
  platform: string
  data_size_bytes: number
  last_dispatch_at?: string
  backend: string
  backend_active: boolean
}
