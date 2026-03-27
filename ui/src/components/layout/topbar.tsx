import { useLocation, useNavigate } from 'react-router-dom'
import { MagnifyingGlass, Plus } from '@phosphor-icons/react'
import { Button } from '../ui/button'

const pathLabels: Record<string, string> = {
  '/': 'dashboard',
  '/jobs': 'jobs',
  '/jobs/new': 'jobs/new',
  '/history': 'history',
  '/agents': 'agents',
  '/settings': 'settings',
}

export function Topbar() {
  const location = useLocation()
  const navigate = useNavigate()

  const label = pathLabels[location.pathname] ?? location.pathname.slice(1)

  return (
    <header className="h-14 border-b border-border bg-bg-base/90 backdrop-blur-md flex items-center justify-between px-6 shrink-0">
      <div className="flex items-center gap-2 font-mono text-sm text-text-muted">
        <span className="text-text-main">~/</span>
        {label}
      </div>

      <div className="flex items-center gap-4">
        <div className="relative group">
          <MagnifyingGlass
            size={16}
            className="absolute left-3 top-1/2 -translate-y-1/2 text-text-muted group-focus-within:text-accent transition-colors"
          />
          <input
            type="text"
            placeholder="Search jobs..."
            className="bg-bg-input border border-border text-sm font-mono text-text-main placeholder:text-text-muted py-1.5 pl-9 pr-3 w-64"
          />
          <div className="absolute right-2 top-1/2 -translate-y-1/2 flex items-center gap-1">
            <kbd className="font-mono text-[10px] text-text-muted bg-bg-base border border-border px-1 py-0.5 rounded-sm">⌘</kbd>
            <kbd className="font-mono text-[10px] text-text-muted bg-bg-base border border-border px-1 py-0.5 rounded-sm">K</kbd>
          </div>
        </div>

        <div className="h-4 w-px bg-border" />

        <Button variant="primary" onClick={() => navigate('/jobs/new')}>
          <Plus size={14} className="mr-1.5" />
          Add Job
        </Button>
      </div>
    </header>
  )
}
