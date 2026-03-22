import { useEffect, useState } from 'react'
import type { FormEvent } from 'react'
import { Link, useNavigate, useParams } from 'react-router-dom'

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

type AccountBalance = {
  currency: string
  amount: string
  updated_at: string
}

type AccountDetail = {
  id: number
  name: string
  account_type: string
  base_currency: string
  created_at: string
  balances: AccountBalance[]
}

type ApiErrorResponse = {
  error: string
  message: string
}

function getAccountsApiUrl() {
  const baseUrl =
    import.meta.env.VITE_API_BASE_URL?.trim() || 'http://127.0.0.1:3000'

  return new URL('/accounts', baseUrl).toString()
}

function getCreateAccountApiUrl() {
  return getAccountsApiUrl()
}

function getAccountDetailApiUrl(accountId: string) {
  return new URL(`/accounts/${accountId}`, getAccountsApiUrl()).toString()
}

function getAccountBalanceApiUrl(accountId: string, currency: string) {
  return new URL(
    `/accounts/${accountId}/balances/${currency}`,
    getAccountsApiUrl()
  ).toString()
}

export function AccountsListPage() {
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

export function AccountDetailPage() {
  const { accountId } = useParams<{ accountId: string }>()
  const [requestState, setRequestState] = useState<
    | { status: 'loading' }
    | { status: 'error'; message: string }
    | { status: 'ready'; account: AccountDetail }
  >({ status: 'loading' })
  const [retryToken, setRetryToken] = useState(0)

  useEffect(() => {
    if (!accountId) {
      setRequestState({ status: 'error', message: 'Account not found.' })
      return
    }

    const resolvedAccountId = accountId

    let cancelled = false

    async function loadAccount() {
      setRequestState({ status: 'loading' })

      try {
        const response = await fetch(getAccountDetailApiUrl(resolvedAccountId))

        if (!response.ok) {
          let message = 'Could not load account.'

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

        const data = (await response.json()) as AccountDetail

        if (!cancelled) {
          setRequestState({ status: 'ready', account: data })
        }
      } catch (error) {
        if (!cancelled) {
          setRequestState({
            status: 'error',
            message:
              error instanceof Error ? error.message : 'Could not load account.',
          })
        }
      }
    }

    void loadAccount()

    return () => {
      cancelled = true
    }
  }, [accountId, retryToken])

  if (requestState.status === 'loading') {
    return (
      <main className="min-h-svh bg-muted/30 px-6 py-10">
        <div className="mx-auto flex w-full max-w-4xl flex-col gap-6">
          <header className="rounded-2xl border bg-background p-6 shadow-sm">
            <p className="text-sm font-medium uppercase tracking-[0.22em] text-muted-foreground">
              Cash Accounts
            </p>
            <h1 className="mt-2 text-3xl font-semibold tracking-tight">
              Account Detail
            </h1>
          </header>
          <div className="grid gap-3">
            {Array.from({ length: 2 }).map((_, index) => (
              <Card key={index} className="border-dashed bg-background/70">
                <CardHeader>
                  <div className="h-5 w-40 rounded-full bg-muted" />
                  <div className="h-4 w-24 rounded-full bg-muted" />
                </CardHeader>
                <CardContent>
                  <div className="h-4 w-32 rounded-full bg-muted" />
                </CardContent>
              </Card>
            ))}
          </div>
        </div>
      </main>
    )
  }

  if (requestState.status === 'error') {
    return (
      <main className="min-h-svh bg-muted/30 px-6 py-10">
        <div className="mx-auto flex w-full max-w-3xl flex-col gap-6">
          <header className="flex items-start justify-between gap-4 rounded-2xl border bg-background p-6 shadow-sm">
            <div>
              <p className="text-sm font-medium uppercase tracking-[0.22em] text-muted-foreground">
                Cash Accounts
              </p>
              <h1 className="mt-2 text-3xl font-semibold tracking-tight">
                Account Detail
              </h1>
            </div>
            <Link className={cn(buttonVariants({ variant: 'outline' }))} to="/accounts">
              Back to accounts
            </Link>
          </header>
          <Card className="border-destructive/30 bg-background">
            <CardHeader>
              <CardTitle>Could not load account</CardTitle>
              <CardDescription>{requestState.message}</CardDescription>
            </CardHeader>
            <CardFooter className="justify-end gap-3">
              <Link
                className={cn(buttonVariants({ variant: 'outline' }))}
                to="/accounts"
              >
                Back to accounts
              </Link>
              <Button onClick={() => setRetryToken((value) => value + 1)} type="button">
                Retry
              </Button>
            </CardFooter>
          </Card>
        </div>
      </main>
    )
  }

  return (
    <AccountDetailReadyState
      account={requestState.account}
      onRefresh={() => setRetryToken((value) => value + 1)}
    />
  )
}

export function AccountNewPage() {
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

function AccountDetailReadyState({
  account,
  onRefresh,
}: {
  account: AccountDetail
  onRefresh: () => void
}) {
  const [currency, setCurrency] = useState(account.base_currency)
  const [amount, setAmount] = useState('')
  const [requestState, setRequestState] = useState<
    | { status: 'idle' }
    | { status: 'submitting' }
    | { status: 'error'; message: string }
  >({ status: 'idle' })
  const [deletingCurrency, setDeletingCurrency] = useState<string | null>(null)

  async function handleBalanceSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()

    const normalizedCurrency = currency.trim().toUpperCase()

    setRequestState({ status: 'submitting' })

    try {
      const response = await fetch(
        getAccountBalanceApiUrl(String(account.id), normalizedCurrency),
        {
          method: 'PUT',
          headers: {
            'Content-Type': 'application/json',
          },
          body: JSON.stringify({
            amount: amount.trim(),
          }),
        }
      )

      if (!response.ok) {
        let message = 'Could not save balance.'

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

      setAmount('')
      setRequestState({ status: 'idle' })
      onRefresh()
    } catch (error) {
      setRequestState({
        status: 'error',
        message:
          error instanceof Error ? error.message : 'Could not save balance.',
      })
    }
  }

  async function handleDeleteBalance(balanceCurrency: string) {
    setDeletingCurrency(balanceCurrency)
    setRequestState({ status: 'idle' })

    try {
      const response = await fetch(
        getAccountBalanceApiUrl(String(account.id), balanceCurrency),
        {
          method: 'DELETE',
        }
      )

      if (!response.ok) {
        let message = 'Could not delete balance.'

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

      onRefresh()
    } catch (error) {
      setRequestState({
        status: 'error',
        message:
          error instanceof Error ? error.message : 'Could not delete balance.',
      })
    } finally {
      setDeletingCurrency(null)
    }
  }

  return (
    <main className="min-h-svh bg-muted/30 px-6 py-10">
      <div className="mx-auto flex w-full max-w-4xl flex-col gap-6">
        <header className="flex flex-col gap-4 rounded-2xl border bg-background p-6 shadow-sm sm:flex-row sm:items-start sm:justify-between">
          <div className="space-y-2">
            <p className="text-sm font-medium uppercase tracking-[0.22em] text-muted-foreground">
              Cash Accounts
            </p>
            <h1 className="text-3xl font-semibold tracking-tight">{account.name}</h1>
            <p className="text-sm text-muted-foreground">
              {account.account_type} · base currency {account.base_currency}
            </p>
          </div>
          <Link className={cn(buttonVariants({ variant: 'outline' }))} to="/accounts">
            Back to accounts
          </Link>
        </header>

        <Card className="bg-background">
          <CardHeader>
            <CardTitle>Account Summary</CardTitle>
            <CardDescription>Created at {account.created_at}</CardDescription>
          </CardHeader>
          <CardContent className="grid gap-3 text-sm sm:grid-cols-3">
            <div className="rounded-xl border p-4">
              <p className="text-muted-foreground">Account type</p>
              <p className="mt-2 font-medium">{account.account_type}</p>
            </div>
            <div className="rounded-xl border p-4">
              <p className="text-muted-foreground">Base currency</p>
              <p className="mt-2 font-medium">{account.base_currency}</p>
            </div>
            <div className="rounded-xl border p-4">
              <p className="text-muted-foreground">Balances</p>
              <p className="mt-2 font-medium">{account.balances.length}</p>
            </div>
          </CardContent>
        </Card>

        <section className="space-y-4">
          <div className="space-y-1">
            <h2 className="text-xl font-semibold tracking-tight">Balances</h2>
            <p className="text-sm text-muted-foreground">
              Current cash balance state for this account.
            </p>
          </div>

          <Card className="bg-background">
            <CardHeader>
              <CardTitle>Update Balance</CardTitle>
              <CardDescription>
                Create or update the current balance for one currency.
              </CardDescription>
            </CardHeader>
            <CardContent>
              <form className="space-y-4" onSubmit={handleBalanceSubmit}>
                <div className="grid gap-4 sm:grid-cols-2">
                  <div className="space-y-2">
                    <label className="text-sm font-medium" htmlFor="balance-currency">
                      Currency
                    </label>
                    <input
                      className="flex h-10 w-full rounded-lg border border-input bg-transparent px-3 py-2 text-sm uppercase outline-none transition-colors placeholder:text-muted-foreground focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50"
                      id="balance-currency"
                      maxLength={3}
                      onChange={(event) =>
                        setCurrency(event.target.value.toUpperCase())
                      }
                      placeholder="USD"
                      required
                      type="text"
                      value={currency}
                    />
                  </div>
                  <div className="space-y-2">
                    <label className="text-sm font-medium" htmlFor="balance-amount">
                      Amount
                    </label>
                    <input
                      className="flex h-10 w-full rounded-lg border border-input bg-transparent px-3 py-2 text-sm outline-none transition-colors placeholder:text-muted-foreground focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50"
                      id="balance-amount"
                      onChange={(event) => setAmount(event.target.value)}
                      placeholder="12000.00000000"
                      required
                      type="text"
                      value={amount}
                    />
                  </div>
                </div>

                {requestState.status === 'error' ? (
                  <p className="text-sm text-destructive">{requestState.message}</p>
                ) : null}

                <div className="flex justify-end">
                  <Button disabled={requestState.status === 'submitting'} type="submit">
                    {requestState.status === 'submitting'
                      ? 'Saving...'
                      : 'Save balance'}
                  </Button>
                </div>
              </form>
            </CardContent>
          </Card>

          {account.balances.length === 0 ? (
            <Card className="border-dashed bg-background">
              <CardHeader>
                <CardTitle>No balances yet</CardTitle>
                <CardDescription>
                  This account does not have any stored balances yet.
                </CardDescription>
              </CardHeader>
            </Card>
          ) : (
            <div className="grid gap-3">
              {account.balances.map((balance) => (
                <Card className="bg-background" key={balance.currency}>
                  <CardHeader>
                    <CardTitle>{balance.currency}</CardTitle>
                    <CardDescription>Updated at {balance.updated_at}</CardDescription>
                    <CardAction>
                      <Button
                        disabled={deletingCurrency === balance.currency}
                        onClick={() => void handleDeleteBalance(balance.currency)}
                        type="button"
                        variant="outline"
                      >
                        {deletingCurrency === balance.currency
                          ? 'Deleting...'
                          : 'Delete'}
                      </Button>
                    </CardAction>
                  </CardHeader>
                  <CardContent>
                    <p className="text-2xl font-semibold tracking-tight">
                      {balance.amount}
                    </p>
                  </CardContent>
                </Card>
              ))}
            </div>
          )}
        </section>
      </div>
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
