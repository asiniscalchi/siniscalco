import { useEffect, useState } from 'react'
import type { FormEvent } from 'react'
import { Link, useParams } from 'react-router-dom'

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
import {
  getAccountBalanceApiUrl,
  getAccountDetailApiUrl,
  readApiErrorMessage,
} from '@/lib/api'
import { cn } from '@/lib/utils'

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
          const message = await readApiErrorMessage(
            response,
            'Could not load account.'
          )
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

  useEffect(() => {
    setCurrency(account.base_currency)
    setAmount('')
    setRequestState({ status: 'idle' })
    setDeletingCurrency(null)
  }, [account.id, account.base_currency])

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
        const message = await readApiErrorMessage(
          response,
          'Could not save balance.'
        )
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
        const message = await readApiErrorMessage(
          response,
          'Could not delete balance.'
        )
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
