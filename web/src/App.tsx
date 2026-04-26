import { Navigate, Route, Routes } from "react-router-dom";

import { AppShell } from "./components/AppShell";
import {
  ActivityPage,
  AssistantPage,
  AccountDetailPage,
  AccountNewPage,
  AccountsListPage,
  AssetDetailPage,
  AssetsPage,
  PortfolioPage,
  SettingsPage,
  TodosPage,
} from "./pages";

function App() {
  return (
    <Routes>
      <Route path="/" element={<Navigate to="/portfolio" replace />} />
      <Route element={<AppShell />}>
        <Route path="/assistant" element={<AssistantPage />} />
        <Route path="/portfolio" element={<PortfolioPage />} />
        <Route path="/todos" element={<TodosPage />} />
        <Route path="/accounts" element={<AccountsListPage />} />
        <Route path="/accounts/new" element={<AccountNewPage />} />
        <Route path="/accounts/:accountId" element={<AccountDetailPage />} />
        <Route path="/assets" element={<AssetsPage />} />
        <Route path="/assets/:assetId" element={<AssetDetailPage />} />
        <Route path="/activity" element={<ActivityPage />} />
        <Route path="/settings" element={<SettingsPage />} />
      </Route>
    </Routes>
  );
}

export default App;
