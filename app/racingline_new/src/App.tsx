import { Navigate, Route, Routes } from "react-router-dom"

import { AppShell } from "@/components/racingline/app-shell"
import { DashboardPage } from "@/routes/dashboard-page"
import { StrategyPage } from "@/routes/strategy-page"

export function App() {
  return (
    <Routes>
      <Route element={<AppShell />}>
        <Route index element={<Navigate replace to="/dashboard" />} />
        <Route path="/dashboard" element={<DashboardPage />} />
        <Route path="/strategies" element={<StrategyPage />} />
      </Route>
      <Route path="*" element={<Navigate replace to="/dashboard" />} />
    </Routes>
  )
}

export default App
