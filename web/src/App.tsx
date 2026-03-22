import { Link, Navigate, Route, Routes } from 'react-router-dom'

import { Button } from '@/components/ui/button'
import {
  Card,
  CardAction,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from '@/components/ui/card'
import { buttonVariants } from '@/components/ui/button-variants'
import { cn } from '@/lib/utils'

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
  const state = getAccountsPageState()

  return (
    <main className="min-h-svh bg-muted/30 px-6 py-10">
      <div className="mx-auto flex w-full max-w-4xl flex-col gap-6">
        <header className="flex flex-col gap-4 rounded-2xl border bg-background p-6 shadow-sm sm:flex-row sm:items-end sm:justify-between">
          <div className="space-y-2">
            <p className="text-sm font-medium uppercase tracking-[0.22em] text-muted-foreground">
              Cash Accounts
            </p>
            <h1 className="text-3xl font-semibold tracking-tight">Accounts</h1>
            <p className="max-w-2xl text-sm text-muted-foreground">
              View your cash accounts and move into account detail or account
              creation.
            </p>
          </div>
          <Link
            className={cn(buttonVariants({ size: 'lg' }))}
            to="/accounts/new"
          >
            Create account
          </Link>
        </header>

        <section className="space-y-4">
          {state === 'loading' ? <AccountsLoadingState /> : null}
          {state === 'empty' ? <AccountsEmptyState /> : null}
          {state === 'error' ? <AccountsErrorState /> : null}
          {state === 'ready' ? <AccountsReadyState /> : null}
        </section>
      </div>
    </main>
  )
}

function getAccountsPageState(): 'loading' | 'empty' | 'error' | 'ready' {
  return 'loading'
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

function AccountsLoadingState() {
  return (
    <div className="grid gap-3">
      {Array.from({ length: 3 }).map((_, index) => (
        <Card key={index} className="border-dashed bg-background/70">
          <CardHeader>
            <div className="h-5 w-32 rounded-full bg-muted" />
            <div className="h-4 w-24 rounded-full bg-muted" />
          </CardHeader>
          <CardContent>
            <div className="h-4 w-20 rounded-full bg-muted" />
          </CardContent>
        </Card>
      ))}
    </div>
  )
}

function AccountsEmptyState() {
  return (
    <Card className="border-dashed bg-background">
      <CardHeader>
        <CardTitle>No accounts yet</CardTitle>
        <CardDescription>
          Create your first cash account to start managing account details.
        </CardDescription>
      </CardHeader>
      <CardFooter className="justify-end">
        <Link className={cn(buttonVariants())} to="/accounts/new">
          Create account
        </Link>
      </CardFooter>
    </Card>
  )
}

function AccountsErrorState() {
  return (
    <Card className="border-destructive/30 bg-background">
      <CardHeader>
        <CardTitle>Could not load accounts</CardTitle>
        <CardDescription>
          The accounts list request failed. Try again to reload the page data.
        </CardDescription>
      </CardHeader>
      <CardFooter className="justify-end gap-3">
        <Link
          className={cn(buttonVariants({ variant: 'outline' }))}
          to="/accounts/new"
        >
          Create account
        </Link>
        <Button type="button">Retry</Button>
      </CardFooter>
    </Card>
  )
}

function AccountsReadyState() {
  return (
    <div className="grid gap-3">
      <AccountListItem
        id="1"
        name="IBKR"
        accountType="broker"
        baseCurrency="EUR"
      />
      <AccountListItem
        id="2"
        name="Main Bank"
        accountType="bank"
        baseCurrency="USD"
      />
    </div>
  )
}

function AccountListItem({
  id,
  name,
  accountType,
  baseCurrency,
}: {
  id: string
  name: string
  accountType: string
  baseCurrency: string
}) {
  return (
    <Card className="bg-background transition-colors hover:bg-muted/30">
      <CardHeader>
        <CardTitle>{name}</CardTitle>
        <CardDescription>{accountType}</CardDescription>
        <CardAction>
          <Link
            className={cn(buttonVariants({ variant: 'outline' }))}
            to={`/accounts/${id}`}
          >
            Open
          </Link>
        </CardAction>
      </CardHeader>
      <CardContent>
        <p className="text-sm text-muted-foreground">{baseCurrency}</p>
      </CardContent>
    </Card>
  )
}

export default App
