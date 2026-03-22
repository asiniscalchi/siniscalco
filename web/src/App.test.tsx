import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { MemoryRouter } from 'react-router-dom'
import { describe, expect, it } from 'vitest'
import { afterEach, beforeEach, vi } from 'vitest'

import App from './App'

describe('App', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  afterEach(() => {
    cleanup()
    vi.unstubAllGlobals()
    vi.restoreAllMocks()
  })

  it('shows a loading state before accounts resolve', () => {
    vi.mocked(fetch).mockImplementation(() => new Promise(() => {}))

    render(
      <MemoryRouter initialEntries={['/accounts']}>
        <App />
      </MemoryRouter>
    )

    expect(screen.getByText('Accounts')).toBeTruthy()
    expect(screen.getByText('Create account')).toBeTruthy()
    expect(screen.getByText('Cash Accounts')).toBeTruthy()
    expect(screen.getByText('Accounts')).toBeTruthy()
    expect(document.querySelectorAll('[data-slot="card"]').length).toBeGreaterThan(0)
  })

  it('renders fetched account summaries', async () => {
    vi.mocked(fetch).mockResolvedValue(
      new Response(
        JSON.stringify([
          {
            id: 1,
            name: 'IBKR',
            account_type: 'broker',
            base_currency: 'EUR',
            created_at: '2026-03-22 00:00:00',
          },
        ]),
        { status: 200, headers: { 'Content-Type': 'application/json' } }
      )
    )

    render(
      <MemoryRouter initialEntries={['/accounts']}>
        <App />
      </MemoryRouter>
    )

    expect(await screen.findByText('IBKR')).toBeTruthy()
    expect(screen.getByText('broker')).toBeTruthy()
    expect(screen.getByText('EUR')).toBeTruthy()
    expect(screen.getByRole('link', { name: 'Open' }).getAttribute('href')).toBe(
      '/accounts/1'
    )
  })

  it('renders the empty state when no accounts exist', async () => {
    vi.mocked(fetch).mockResolvedValue(
      new Response(JSON.stringify([]), {
        status: 200,
        headers: { 'Content-Type': 'application/json' },
      })
    )

    render(
      <MemoryRouter initialEntries={['/accounts']}>
        <App />
      </MemoryRouter>
    )

    expect(await screen.findByText('No accounts yet')).toBeTruthy()
  })

  it('renders an error state and retries the request', async () => {
    vi.mocked(fetch)
      .mockRejectedValueOnce(new Error('network error'))
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify([
            {
              id: 1,
              name: 'Main Bank',
              account_type: 'bank',
              base_currency: 'USD',
              created_at: '2026-03-22 00:00:00',
            },
          ]),
          { status: 200, headers: { 'Content-Type': 'application/json' } }
        )
      )

    render(
      <MemoryRouter initialEntries={['/accounts']}>
        <App />
      </MemoryRouter>
    )

    expect(await screen.findByText('Could not load accounts')).toBeTruthy()

    fireEvent.click(screen.getByText('Retry'))

    await waitFor(() => {
      expect(screen.getByText('Main Bank')).toBeTruthy()
    })
    expect(fetch).toHaveBeenCalledTimes(2)
  })

  it('navigates from an account item to the account detail route', async () => {
    vi.mocked(fetch)
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify([
            {
              id: 7,
              name: 'IBKR',
              account_type: 'broker',
              base_currency: 'EUR',
              created_at: '2026-03-22 00:00:00',
            },
          ]),
          { status: 200, headers: { 'Content-Type': 'application/json' } }
        )
      )
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify({
            id: 7,
            name: 'IBKR',
            account_type: 'broker',
            base_currency: 'EUR',
            created_at: '2026-03-22 00:00:00',
            balances: [
              {
                currency: 'USD',
                amount: '12.30000000',
                updated_at: '2026-03-22 00:00:00',
              },
            ],
          }),
          { status: 200, headers: { 'Content-Type': 'application/json' } }
        )
      )

    render(
      <MemoryRouter initialEntries={['/accounts']}>
        <App />
      </MemoryRouter>
    )

    fireEvent.click(await screen.findByRole('link', { name: 'Open' }))

    expect(await screen.findByText('IBKR')).toBeTruthy()
    expect(screen.getByText('broker · base currency EUR')).toBeTruthy()
    expect(screen.getByText('12.30000000')).toBeTruthy()
  })

  it('navigates from the create account action to the new account route', async () => {
    vi.mocked(fetch).mockResolvedValue(
      new Response(JSON.stringify([]), {
        status: 200,
        headers: { 'Content-Type': 'application/json' },
      })
    )

    render(
      <MemoryRouter initialEntries={['/accounts']}>
        <App />
      </MemoryRouter>
    )

    fireEvent.click(await screen.findByRole('link', { name: 'Create account' }))

    expect(await screen.findByText('New Account')).toBeTruthy()
    expect(screen.getByLabelText('Name')).toBeTruthy()
    expect(screen.getByLabelText('Account type')).toBeTruthy()
    expect(screen.getByLabelText('Base currency')).toBeTruthy()
  })

  it('creates an account and returns to the accounts list', async () => {
    vi.mocked(fetch)
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify({
            id: 12,
            name: 'IBKR',
            account_type: 'broker',
            base_currency: 'EUR',
            created_at: '2026-03-22 00:00:00',
          }),
          { status: 201, headers: { 'Content-Type': 'application/json' } }
        )
      )
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify([
            {
              id: 12,
              name: 'IBKR',
              account_type: 'broker',
              base_currency: 'EUR',
              created_at: '2026-03-22 00:00:00',
            },
          ]),
          { status: 200, headers: { 'Content-Type': 'application/json' } }
        )
      )

    render(
      <MemoryRouter initialEntries={['/accounts/new']}>
        <App />
      </MemoryRouter>
    )

    fireEvent.change(screen.getByLabelText('Name'), {
      target: { value: 'IBKR' },
    })
    fireEvent.change(screen.getByLabelText('Account type'), {
      target: { value: 'broker' },
    })
    fireEvent.change(screen.getByLabelText('Base currency'), {
      target: { value: 'eur' },
    })

    fireEvent.click(screen.getByRole('button', { name: 'Create account' }))

    expect(await screen.findByText('Accounts')).toBeTruthy()
    expect(await screen.findByText('IBKR')).toBeTruthy()
    expect(fetch).toHaveBeenNthCalledWith(
      1,
      'http://127.0.0.1:3000/accounts',
      expect.objectContaining({
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          name: 'IBKR',
          account_type: 'broker',
          base_currency: 'EUR',
        }),
      })
    )
  })

  it('shows an API error when account creation fails', async () => {
    vi.mocked(fetch).mockResolvedValue(
      new Response(
        JSON.stringify({
          error: 'validation_error',
          message: 'Invalid currency format',
        }),
        { status: 400, headers: { 'Content-Type': 'application/json' } }
      )
    )

    render(
      <MemoryRouter initialEntries={['/accounts/new']}>
        <App />
      </MemoryRouter>
    )

    fireEvent.change(screen.getByLabelText('Name'), {
      target: { value: 'IBKR' },
    })
    fireEvent.click(screen.getByRole('button', { name: 'Create account' }))

    expect(await screen.findByText('Invalid currency format')).toBeTruthy()
  })

  it('renders account detail with empty balances', async () => {
    vi.mocked(fetch).mockResolvedValue(
      new Response(
        JSON.stringify({
          id: 3,
          name: 'Main Bank',
          account_type: 'bank',
          base_currency: 'USD',
          created_at: '2026-03-22 00:00:00',
          balances: [],
        }),
        { status: 200, headers: { 'Content-Type': 'application/json' } }
      )
    )

    render(
      <MemoryRouter initialEntries={['/accounts/3']}>
        <App />
      </MemoryRouter>
    )

    expect(await screen.findByText('Main Bank')).toBeTruthy()
    expect(screen.getByText('No balances yet')).toBeTruthy()
    expect(fetch).toHaveBeenCalledWith('http://127.0.0.1:3000/accounts/3')
  })

  it('renders an account detail error and retries the request', async () => {
    vi.mocked(fetch)
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify({
            error: 'not_found',
            message: 'Account not found',
          }),
          { status: 404, headers: { 'Content-Type': 'application/json' } }
        )
      )
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify({
            id: 8,
            name: 'Broker',
            account_type: 'broker',
            base_currency: 'EUR',
            created_at: '2026-03-22 00:00:00',
            balances: [
              {
                currency: 'EUR',
                amount: '100.00000000',
                updated_at: '2026-03-22 00:00:00',
              },
            ],
          }),
          { status: 200, headers: { 'Content-Type': 'application/json' } }
        )
      )

    render(
      <MemoryRouter initialEntries={['/accounts/8']}>
        <App />
      </MemoryRouter>
    )

    expect(await screen.findByText('Could not load account')).toBeTruthy()
    expect(screen.getByText('Account not found')).toBeTruthy()

    fireEvent.click(screen.getByRole('button', { name: 'Retry' }))

    expect(await screen.findByText('Broker')).toBeTruthy()
    expect(screen.getByText('100.00000000')).toBeTruthy()
    expect(fetch).toHaveBeenCalledTimes(2)
  })

  it('upserts a balance from the account detail page', async () => {
    vi.mocked(fetch)
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify({
            id: 9,
            name: 'IBKR',
            account_type: 'broker',
            base_currency: 'EUR',
            created_at: '2026-03-22 00:00:00',
            balances: [],
          }),
          { status: 200, headers: { 'Content-Type': 'application/json' } }
        )
      )
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify({
            currency: 'USD',
            amount: '42.50000000',
            updated_at: '2026-03-22 00:00:00',
          }),
          { status: 201, headers: { 'Content-Type': 'application/json' } }
        )
      )
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify({
            id: 9,
            name: 'IBKR',
            account_type: 'broker',
            base_currency: 'EUR',
            created_at: '2026-03-22 00:00:00',
            balances: [
              {
                currency: 'USD',
                amount: '42.50000000',
                updated_at: '2026-03-22 00:00:00',
              },
            ],
          }),
          { status: 200, headers: { 'Content-Type': 'application/json' } }
        )
      )

    render(
      <MemoryRouter initialEntries={['/accounts/9']}>
        <App />
      </MemoryRouter>
    )

    expect(await screen.findByText('No balances yet')).toBeTruthy()

    fireEvent.change(screen.getByLabelText('Currency'), {
      target: { value: 'usd' },
    })
    fireEvent.change(screen.getByLabelText('Amount'), {
      target: { value: '42.5' },
    })
    fireEvent.click(screen.getByRole('button', { name: 'Save balance' }))

    expect(await screen.findByText('42.50000000')).toBeTruthy()
    expect(fetch).toHaveBeenNthCalledWith(
      2,
      'http://127.0.0.1:3000/accounts/9/balances/USD',
      expect.objectContaining({
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ amount: '42.5' }),
      })
    )
  })

  it('deletes a balance from the account detail page', async () => {
    vi.mocked(fetch)
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify({
            id: 10,
            name: 'Main Bank',
            account_type: 'bank',
            base_currency: 'USD',
            created_at: '2026-03-22 00:00:00',
            balances: [
              {
                currency: 'USD',
                amount: '100.00000000',
                updated_at: '2026-03-22 00:00:00',
              },
            ],
          }),
          { status: 200, headers: { 'Content-Type': 'application/json' } }
        )
      )
      .mockResolvedValueOnce(new Response(null, { status: 204 }))
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify({
            id: 10,
            name: 'Main Bank',
            account_type: 'bank',
            base_currency: 'USD',
            created_at: '2026-03-22 00:00:00',
            balances: [],
          }),
          { status: 200, headers: { 'Content-Type': 'application/json' } }
        )
      )

    render(
      <MemoryRouter initialEntries={['/accounts/10']}>
        <App />
      </MemoryRouter>
    )

    expect(await screen.findByText('100.00000000')).toBeTruthy()

    fireEvent.click(screen.getByRole('button', { name: 'Delete' }))

    expect(await screen.findByText('No balances yet')).toBeTruthy()
    expect(fetch).toHaveBeenNthCalledWith(
      2,
      'http://127.0.0.1:3000/accounts/10/balances/USD',
      expect.objectContaining({
        method: 'DELETE',
      })
    )
  })
})
