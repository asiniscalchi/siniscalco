import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { MemoryRouter, Route, Routes } from 'react-router-dom'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import { AccountDetailPage } from './AccountDetailPage'

describe('AccountDetailPage', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  afterEach(() => {
    cleanup()
    vi.unstubAllGlobals()
    vi.restoreAllMocks()
  })

  it('renders account detail with balances', async () => {
    vi.mocked(fetch).mockResolvedValue(
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
      <MemoryRouter initialEntries={['/accounts/7']}>
        <Routes>
          <Route path="/accounts/:accountId" element={<AccountDetailPage />} />
        </Routes>
      </MemoryRouter>
    )

    expect(await screen.findByText('IBKR')).toBeTruthy()
    expect(screen.getByText('broker · base currency EUR')).toBeTruthy()
    expect(screen.getByText('12.30000000')).toBeTruthy()
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
        <Routes>
          <Route path="/accounts/:accountId" element={<AccountDetailPage />} />
        </Routes>
      </MemoryRouter>
    )

    expect(await screen.findByText('Main Bank')).toBeTruthy()
    expect(screen.getByText('No balances yet')).toBeTruthy()
    expect(fetch).toHaveBeenCalledWith('http://127.0.0.1:3000/accounts/3')
  })

  it('renders an error state and retries the request', async () => {
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
        <Routes>
          <Route path="/accounts/:accountId" element={<AccountDetailPage />} />
        </Routes>
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
        <Routes>
          <Route path="/accounts/:accountId" element={<AccountDetailPage />} />
        </Routes>
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
        <Routes>
          <Route path="/accounts/:accountId" element={<AccountDetailPage />} />
        </Routes>
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

  it('resets the balance form when the loaded account changes', async () => {
    vi.mocked(fetch)
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify({
            id: 11,
            name: 'First Account',
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
            id: 12,
            name: 'Second Account',
            account_type: 'bank',
            base_currency: 'USD',
            created_at: '2026-03-22 00:00:00',
            balances: [],
          }),
          { status: 200, headers: { 'Content-Type': 'application/json' } }
        )
      )

    const firstRender = render(
      <MemoryRouter initialEntries={['/accounts/11']}>
        <Routes>
          <Route path="/accounts/:accountId" element={<AccountDetailPage />} />
        </Routes>
      </MemoryRouter>
    )

    expect(await screen.findByText('First Account')).toBeTruthy()

    fireEvent.change(screen.getByLabelText('Currency'), {
      target: { value: 'GBP' },
    })
    fireEvent.change(screen.getByLabelText('Amount'), {
      target: { value: '99.5' },
    })

    firstRender.unmount()

    render(
      <MemoryRouter initialEntries={['/accounts/12']}>
        <Routes>
          <Route path="/accounts/:accountId" element={<AccountDetailPage />} />
        </Routes>
      </MemoryRouter>
    )

    expect(await screen.findByText('Second Account')).toBeTruthy()
    expect((screen.getByLabelText('Currency') as HTMLInputElement).value).toBe('USD')
    expect((screen.getByLabelText('Amount') as HTMLInputElement).value).toBe('')
  })
})
