import { useEffect, useState } from 'react'
import type { FormEvent } from 'react'
import { Link, Navigate, Route, Routes, useNavigate, useParams } from 'react-router-dom'

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

type AccountSummary = {
  id: number
  name: string
  account_type: string
  base_currency: string
  created_at: string
}

type ApiErrorResponse = {
  error: string
  message: string
}

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
  const [requestState, setRequestState] = useState<
    | { status: 'loading' }
    | { status: 'empty' }
    | { status: 'error' }
    | { status: 'ready'; accounts: AccountSummary[] }
  >({ status: 'loading' })
  const [retryToken, setRetryToken] = useState(0)

  useEffect(() => {
    let cancelled = false

    async function loadAccounts() {
      setRequestState({ status: 'loading' })

      try {
        const response = await fetch(getAccountsApiUrl())

        if (!response.ok) {
          throw new Error(`accounts request failed with status ${response.status}`)
        }

        const data = (await response.json()) as AccountSummary[]

        if (cancelled) {
          return
        }

        if (data.length === 0) {
          setRequestState({ status: 'empty' })
          return
        }

        setRequestState({ status: 'ready', accounts: data })
      } catch {
        if (!cancelled) {
          setRequestState({ status: 'error' })
        }
      }
    }

    void loadAccounts()

    return () => {
      cancelled = true
    }
  }, [retryToken])

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
          {requestState.status === 'loading' ? <AccountsLoadingState /> : null}
          {requestState.status === 'empty' ? <AccountsEmptyState /> : null}
          {requestState.status === 'error' ? (
            <AccountsErrorState onRetry={() => setRetryToken((value) => value + 1)} />
          ) : null}
          {requestState.status === 'ready' ? (
            <AccountsReadyState accounts={requestState.accounts} />
          ) : null}
        </section>
      </div>
    </main>
  )
}

function getAccountsApiUrl() {
  const baseUrl =
    import.meta.env.VITE_API_BASE_URL?.trim() || 'http://127.0.0.1:3000'

  return new URL('/accounts', baseUrl).toString()
}

function getCreateAccountApiUrl() {
  return getAccountsApiUrl()
}

function AccountDetailPage() {
  const { accountId } = useParams<{ accountId: string }>()

  return (
    <PageShell
      title="Account Detail"
      description={`Account detail route placeholder for account ${accountId ?? 'unknown'}.`}
    />
  )
}

function AccountNewPage() {
  const navigate = useNavigate()
  const [name, setName] = useState('')
  const [accountType, setAccountType] = useState<'bank' | 'broker'>('bank')
  const [baseCurrency, setBaseCurrency] = useState('EUR')
  const [requestState, setRequestState] = useState<
    | { status: 'idle' }
    | { status: 'submitting' }
    | { status: 'error'; message: string }
  >({ status: 'idle' })

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()

    setRequestState({ status: 'submitting' })

    try {
      const response = await fetch(getCreateAccountApiUrl(), {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          name: name.trim(),
          account_type: accountType,
          base_currency: baseCurrency.trim().toUpperCase(),
        }),
      })

      if (!response.ok) {
        let message = 'Could not create account.'

        try {
          const data = (await response.json()) as ApiErrorResponse
          if (data.message) {
            message = data.message
          }
        } catch {
          // Keep the fallback message when the error body is unavailable.
        }

        throw new Error(message)
      }

      navigate('/accounts')
    } catch (error) {
      setRequestState({
        status: 'error',
        message:
          error instanceof Error ? error.message : 'Could not create account.',
      })
    }
  }

  return (
    <main className="min-h-svh bg-muted/30 px-6 py-10">
      <div className="mx-auto flex w-full max-w-2xl flex-col gap-6">
        <header className="flex flex-col gap-3 rounded-2xl border bg-background p-6 shadow-sm">
          <p className="text-sm font-medium uppercase tracking-[0.22em] text-muted-foreground">
            Cash Accounts
          </p>
          <h1 className="text-3xl font-semibold tracking-tight">New Account</h1>
          <p className="max-w-xl text-sm text-muted-foreground">
            Create a cash account with its name, account type, and base currency.
          </p>
        </header>

        <Card className="bg-background">
          <CardContent className="pt-6">
            <form className="space-y-5" onSubmit={handleSubmit}>
              <div className="space-y-2">
                <label className="text-sm font-medium" htmlFor="account-name">
                  Name
                </label>
                <input
                  className="flex h-10 w-full rounded-lg border border-input bg-transparent px-3 py-2 text-sm outline-none transition-colors placeholder:text-muted-foreground focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50"
                  id="account-name"
                  name="name"
                  onChange={(event) => setName(event.target.value)}
                  placeholder="IBKR"
                  required
                  type="text"
                  value={name}
                />
              </div>

              <div className="space-y-2">
                <label className="text-sm font-medium" htmlFor="account-type">
                  Account type
                </label>
                <select
                  className="flex h-10 w-full rounded-lg border border-input bg-transparent px-3 py-2 text-sm outline-none transition-colors focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50"
                  id="account-type"
                  name="account_type"
                  onChange={(event) =>
                    setAccountType(event.target.value as 'bank' | 'broker')
                  }
                  value={accountType}
                >
                  <option value="bank">bank</option>
                  <option value="broker">broker</option>
                </select>
              </div>

              <div className="space-y-2">
                <label className="text-sm font-medium" htmlFor="base-currency">
                  Base currency
                </label>
                <input
                  className="flex h-10 w-full rounded-lg border border-input bg-transparent px-3 py-2 text-sm uppercase outline-none transition-colors placeholder:text-muted-foreground focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50"
                  id="base-currency"
                  maxLength={3}
                  name="base_currency"
                  onChange={(event) =>
                    setBaseCurrency(event.target.value.toUpperCase())
                  }
                  placeholder="EUR"
                  required
                  type="text"
                  value={baseCurrency}
                />
              </div>

              {requestState.status === 'error' ? (
                <p className="text-sm text-destructive">{requestState.message}</p>
              ) : null}

              <div className="flex justify-end gap-3">
                <Link
                  className={cn(buttonVariants({ variant: 'outline' }))}
                  to="/accounts"
                >
                  Cancel
                </Link>
                <Button disabled={requestState.status === 'submitting'} type="submit">
                  {requestState.status === 'submitting'
                    ? 'Creating...'
                    : 'Create account'}
                </Button>
              </div>
            </form>
          </CardContent>
        </Card>
      </div>
    </main>
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

function AccountsErrorState({ onRetry }: { onRetry: () => void }) {
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
        <Button onClick={onRetry} type="button">
          Retry
        </Button>
      </CardFooter>
    </Card>
  )
}

function AccountsReadyState({ accounts }: { accounts: AccountSummary[] }) {
  return (
    <div className="grid gap-3">
      {accounts.map((account) => (
        <AccountListItem
          key={account.id}
          id={String(account.id)}
          name={account.name}
          accountType={account.account_type}
          baseCurrency={account.base_currency}
        />
      ))}
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
