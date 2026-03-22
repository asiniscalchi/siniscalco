import { Navigate, Route, Routes } from 'react-router-dom'

import {
  AccountDetailPage,
  AccountNewPage,
  AccountsListPage,
} from './pages'

function App() {
  return (
    <Routes>
      <Route path="/" element={<Navigate to="/accounts" replace />} />
      <Route path="/accounts" element={<AccountsListPage />} />
      <Route path="/accounts/new" element={<AccountNewPage />} />
      <Route path="/accounts/:accountId" element={<AccountDetailPage />} />
    </Routes>
  )
}

export default App
