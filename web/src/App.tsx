import { Navigate, Route, Routes } from 'react-router-dom'

import {
  Card,
  CardContent,
  CardHeader,
  CardTitle,
} from '@/components/ui/card'

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

function AccountsListPage() {
  return (
    <PageShell
      title="Accounts"
      description="Accounts list page placeholder."
    />
  )
}

function AccountDetailPage() {
  return (
    <PageShell
      title="Account Detail"
      description="Account detail route placeholder."
    />
  )
}

function AccountNewPage() {
  return (
    <PageShell
      title="New Account"
      description="Account creation route placeholder."
    />
  )
}

function PageShell({
  title,
  description,
}: {
  title: string
  description: string
}) {
  return (
    <main className="flex min-h-svh items-center justify-center p-6">
      <Card className="w-full max-w-md">
        <CardHeader>
          <CardTitle className="text-3xl">{title}</CardTitle>
        </CardHeader>
        <CardContent>
          <p className="text-muted-foreground">{description}</p>
        </CardContent>
      </Card>
    </main>
  )
}

export default App
