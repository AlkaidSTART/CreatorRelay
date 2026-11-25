import { HashRouter, Navigate, Route, Routes } from "react-router";
import { AppLayout } from "./layouts/AppLayout";
import { ModulePlaceholder } from "./components/ModulePlaceholder";
import { PlatformsPage } from "./pages/PlatformsPage";
import { SettingsPage } from "./pages/SettingsPage";
import { StudioPage } from "./pages/StudioPage";
import { ContentStudioPage } from "./pages/ContentStudioPage";

function App() {
  return (
    <HashRouter>
      <Routes>
        <Route element={<AppLayout />}>
          <Route index element={<Navigate to="/dashboard" replace />} />
          <Route path="/dashboard" element={<ModulePlaceholder routeId="dashboard" />} />
          <Route path="/studio" element={<ContentStudioPage />} />
          <Route path="/studio/classic" element={<StudioPage />} />
          <Route path="/publishing" element={<ModulePlaceholder routeId="publishing" />} />
          <Route path="/content" element={<ModulePlaceholder routeId="content" />} />
          <Route path="/analytics" element={<ModulePlaceholder routeId="analytics" />} />
          <Route path="/assistant" element={<ModulePlaceholder routeId="assistant" />} />
          <Route path="/platforms" element={<PlatformsPage />} />
          <Route path="/settings" element={<SettingsPage />} />
          <Route path="*" element={<Navigate to="/studio" replace />} />
        </Route>
      </Routes>
    </HashRouter>
  );
}

export default App;
