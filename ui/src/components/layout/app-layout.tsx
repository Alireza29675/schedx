import { Outlet } from 'react-router-dom'
import { Sidebar } from './sidebar'
import { Topbar } from './topbar'

export function AppLayout() {
  return (
    <div className="h-screen w-screen flex relative">
      <div className="absolute inset-0 pointer-events-none bg-grid z-0" />
      <Sidebar />
      <main className="flex-1 h-full flex flex-col z-10 relative bg-bg-base/50 min-w-0">
        <Topbar />
        <div className="flex-1 overflow-y-auto p-6 relative">
          <Outlet />
        </div>
      </main>
    </div>
  )
}
