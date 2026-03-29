import { Navigate, Route, Routes } from "react-router-dom";

import { AppShell } from "./components/AppShell";
import {
  AccountDetailPage,
  AccountNewPage,
  AccountsListPage,
  AssetsPage,
  PortfolioPage,
  TransactionsPage,
  TransfersPage,
} from "./pages";

function App() {
  return (
    <Routes>
      <Route path="/" element={<Navigate to="/portfolio" replace />} />
      <Route element={<AppShell />}>
        <Route path="/portfolio" element={<PortfolioPage />} />
        <Route path="/accounts" element={<AccountsListPage />} />
        <Route path="/accounts/new" element={<AccountNewPage />} />
        <Route path="/accounts/:accountId" element={<AccountDetailPage />} />
        <Route path="/assets" element={<AssetsPage />} />
        <Route path="/transactions" element={<TransactionsPage />} />
        <Route path="/transfers" element={<TransfersPage />} />
      </Route>
    </Routes>
  );
}

export default App;
