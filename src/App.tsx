import { HashRouter, Navigate, Route, Routes } from "react-router";
import { AppLayout } from "./layouts/AppLayout";
import { AdaptersPage } from "./pages/AdaptersPage";
import { AiSettingsPage } from "./pages/AiSettingsPage";
import { AnalyticsPage } from "./pages/AnalyticsPage";
import { AssistantPage } from "./pages/AssistantPage";
import { ContentPage } from "./pages/ContentPage";
import { ContentStudioPage } from "./pages/ContentStudioPage";
import { CreatorCentersPage } from "./pages/CreatorCentersPage";
import { DashboardPage } from "./pages/DashboardPage";
import { DiagnosticsPage } from "./pages/DiagnosticsPage";
import { EcosystemPage } from "./pages/EcosystemPage";
import { PlatformsPage } from "./pages/PlatformsPage";
import { PublishingPage } from "./pages/PublishingPage";
import { SettingsPage } from "./pages/SettingsPage";
import { StudioPage } from "./pages/StudioPage";

function App() {
  return (
    <HashRouter>
      <Routes>
        <Route element={<AppLayout />}>
          <Route index element={<Navigate to="/dashboard" replace />} />
          <Route path="/dashboard" element={<DashboardPage />} />
          <Route path="/studio" element={<ContentStudioPage />} />
          <Route path="/studio/classic" element={<StudioPage />} />
          <Route path="/publishing" element={<PublishingPage />} />
          <Route path="/content" element={<ContentPage />} />
          <Route path="/analytics" element={<AnalyticsPage />} />
          <Route path="/assistant" element={<AssistantPage />} />
          <Route path="/platforms" element={<PlatformsPage />} />
          <Route path="/adapters" element={<AdaptersPage />} />
          <Route path="/creator-centers" element={<CreatorCentersPage />} />
          <Route path="/ecosystem" element={<EcosystemPage />} />
          <Route path="/settings" element={<SettingsPage />} />
          <Route path="/settings/ai" element={<AiSettingsPage />} />
          <Route path="/settings/diagnostics" element={<DiagnosticsPage />} />
          <Route path="*" element={<Navigate to="/studio" replace />} />
        </Route>
      </Routes>
    </HashRouter>
  );
}

export default App;
