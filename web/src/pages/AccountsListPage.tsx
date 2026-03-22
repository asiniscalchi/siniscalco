import { useEffect, useState } from 'react'
import { Link } from 'react-router-dom'

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
import { getAccountsApiUrl } from '@/lib/api'
import { cn } from '@/lib/utils'

type AccountSummary = {
  id: number
  name: string
  account_type: string
  base_currency: string
  created_at: string
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
