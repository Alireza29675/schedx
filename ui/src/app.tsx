import { Routes, Route, Navigate } from 'react-router-dom'
import { AppLayout } from './components/layout/app-layout'
import DashboardPage from './pages/dashboard'
import JobsListPage from './pages/jobs/jobs-list'
import JobDetailPage from './pages/jobs/job-detail'
import JobFormPage from './pages/jobs/job-form'
import HistoryPage from './pages/history'
import RunDetailPage from './pages/run-detail'
import AgentsPage from './pages/agents'
import SettingsPage from './pages/settings'
import { ToastContainer } from './components/ui/toast-container'

export function App() {
  return (
    <>
      <Routes>
        <Route element={<AppLayout />}>
          <Route path="/" element={<DashboardPage />} />
          <Route path="/jobs" element={<JobsListPage />} />
          <Route path="/jobs/new" element={<JobFormPage />} />
          <Route path="/jobs/:id" element={<JobDetailPage />} />
          <Route path="/jobs/:id/edit" element={<JobFormPage />} />
          <Route path="/history" element={<HistoryPage />} />
          <Route path="/runs/:runId" element={<RunDetailPage />} />
          <Route path="/agents" element={<AgentsPage />} />
          <Route path="/settings" element={<SettingsPage />} />
          <Route path="*" element={<Navigate to="/" replace />} />
        </Route>
      </Routes>
      <ToastContainer />
    </>
  )
}
