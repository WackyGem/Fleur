import { Navigate, Route, Routes } from "react-router-dom"

import { AppShell } from "@/components/racingline/app-shell"
import { MetricsPage } from "@/routes/MetricsPage"
import { PortfolioDetailPage } from "@/routes/PortfolioDetailPage"
import { PortfoliosPage } from "@/routes/PortfoliosPage"
import { RunDetailPage } from "@/routes/RunDetailPage"
import { RunsPage } from "@/routes/RunsPage"
import { RulesPage } from "@/routes/RulesPage"
import { SecurityAnalysisPage } from "@/routes/SecurityAnalysisPage"

export default function App() {
  return (
    <Routes>
      <Route element={<AppShell />}>
        <Route index element={<Navigate replace to="/runs" />} />
        <Route path="runs" element={<RunsPage />} />
        <Route path="runs/:runId" element={<RunDetailPage />} />
        <Route path="portfolios" element={<PortfoliosPage />} />
        <Route
          path="portfolios/:portfolioRunId"
          element={<PortfolioDetailPage />}
        />
        <Route
          path="runs/:runId/securities/:securityCode"
          element={<SecurityAnalysisPage />}
        />
        <Route path="rules" element={<RulesPage />} />
        <Route path="metrics" element={<MetricsPage />} />
        <Route path="*" element={<Navigate replace to="/runs" />} />
      </Route>
    </Routes>
  )
}
