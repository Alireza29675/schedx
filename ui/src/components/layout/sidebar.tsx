import { NavLink, useLocation } from 'react-router-dom'
import {
  SquaresFour,
  ListDashes,
  ClockCounterClockwise,
  Cpu,
  Gear,
  Hexagon,
} from '@phosphor-icons/react'
import { cn } from '../../lib/cn'

const mainNav = [
  { to: '/', icon: SquaresFour, label: 'Dashboard' },
  { to: '/jobs', icon: ListDashes, label: 'Jobs' },
  { to: '/history', icon: ClockCounterClockwise, label: 'History' },
]

const resourceNav = [
  { to: '/agents', icon: Cpu, label: 'Agents' },
]

const bottomNav = [
  { to: '/settings', icon: Gear, label: 'Settings' },
]

interface NavItemProps {
  to: string
  icon: React.ComponentType<{ size?: number; className?: string; weight?: 'fill' | 'regular' | 'bold' | 'light' | 'thin' | 'duotone' }>
  label: string
}

function NavItem({ to, icon: Icon, label }: NavItemProps) {
  const location = useLocation()
  const isActive = to === '/' ? location.pathname === '/' : location.pathname.startsWith(to)

  return (
    <li>
      <NavLink
        to={to}
        className={cn(
          'w-full flex items-center gap-3 px-6 py-2 text-sm border-l-2 transition-colors text-left',
          isActive
            ? 'text-text-main border-accent bg-bg-panel'
            : 'text-text-muted hover:text-text-main hover:bg-bg-panel/50 border-transparent hover:border-border',
        )}
      >
        <Icon
          size={18}
          className={cn(isActive ? 'text-accent' : '')}
          weight={isActive ? 'fill' : 'regular'}
        />
        {label}
      </NavLink>
    </li>
  )
}

function NavGroup({ label, items }: { label: string; items: NavItemProps[] }) {
  return (
    <div>
      <div className="px-6 mb-3 label-mono">{label}</div>
      <ul className="flex flex-col">
        {items.map((item) => (
          <NavItem key={item.to} {...item} />
        ))}
      </ul>
    </div>
  )
}

export function Sidebar() {
  return (
    <aside className="w-[240px] h-full border-r border-border bg-bg-base/90 backdrop-blur-md flex flex-col z-10 relative shrink-0">
      <div className="h-14 border-b border-border flex items-center px-6">
        <NavLink to="/" className="flex items-center gap-2">
          <Hexagon size={20} weight="fill" className="text-accent" />
          <span className="font-heading font-bold text-lg tracking-tight">schedx</span>
        </NavLink>
      </div>

      <nav className="flex-1 py-6 flex flex-col gap-6 overflow-y-auto">
        <NavGroup label="Scheduler" items={mainNav} />
        <NavGroup label="Resources" items={resourceNav} />
        <div className="mt-auto pb-4">
          <ul className="flex flex-col">
            {bottomNav.map((item) => (
              <NavItem key={item.to} {...item} />
            ))}
          </ul>
        </div>
      </nav>
    </aside>
  )
}
