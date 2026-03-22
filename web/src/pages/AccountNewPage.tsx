import { useState } from 'react'
import type { FormEvent } from 'react'
import { Link, useNavigate } from 'react-router-dom'

import { Button } from '@/components/ui/button'
import { Card, CardContent } from '@/components/ui/card'
import { buttonVariants } from '@/components/ui/button-variants'
import { getAccountsApiUrl, readApiErrorMessage } from '@/lib/api'
import { cn } from '@/lib/utils'

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
      const response = await fetch(getAccountsApiUrl(), {
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
        const message = await readApiErrorMessage(
          response,
          'Could not create account.'
        )
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
  )
}
