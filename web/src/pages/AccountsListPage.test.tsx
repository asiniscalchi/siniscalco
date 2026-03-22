import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { MemoryRouter } from 'react-router-dom'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import { AccountsListPage } from './AccountsListPage'

describe('AccountsListPage', () => {
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
      <MemoryRouter>
        <AccountsListPage />
      </MemoryRouter>
    )

    expect(screen.getByText('Accounts')).toBeTruthy()
    expect(screen.getByText('Create account')).toBeTruthy()
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
            summary_status: 'ok',
            total_amount: '123.45000000',
            total_currency: 'EUR',
          },
        ]),
        { status: 200, headers: { 'Content-Type': 'application/json' } }
      )
    )

    render(
      <MemoryRouter>
        <AccountsListPage />
      </MemoryRouter>
    )

    expect(await screen.findByText('IBKR')).toBeTruthy()
    expect(screen.getByText(/broker/)).toBeTruthy()
    expect(screen.getAllByText(/EUR/).length).toBeGreaterThan(0)
    expect(screen.getByText('123.45000000 EUR')).toBeTruthy()
    expect(screen.getByRole('link', { name: /IBKR.*broker.*EUR.*View details/ }).getAttribute('href')).toBe(
      '/accounts/1'
    )
  })

  it('renders conversion unavailable when the backend summary cannot be calculated', async () => {
    vi.mocked(fetch).mockResolvedValue(
      new Response(
        JSON.stringify([
          {
            id: 1,
            name: 'IBKR',
            account_type: 'broker',
            base_currency: 'EUR',
            summary_status: 'conversion_unavailable',
            total_amount: null,
            total_currency: null,
          },
        ]),
        { status: 200, headers: { 'Content-Type': 'application/json' } }
      )
    )

    render(
      <MemoryRouter>
        <AccountsListPage />
      </MemoryRouter>
    )

    expect(await screen.findByText('Conversion unavailable')).toBeTruthy()
  })

  it('renders the empty state when no accounts exist', async () => {
    vi.mocked(fetch).mockResolvedValue(
      new Response(JSON.stringify([]), {
        status: 200,
        headers: { 'Content-Type': 'application/json' },
      })
    )

    render(
      <MemoryRouter>
        <AccountsListPage />
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
              summary_status: 'ok',
              total_amount: '50.00000000',
              total_currency: 'USD',
            },
          ]),
          { status: 200, headers: { 'Content-Type': 'application/json' } }
        )
      )

    render(
      <MemoryRouter>
        <AccountsListPage />
      </MemoryRouter>
    )

    expect(await screen.findByText('Could not load accounts')).toBeTruthy()

    fireEvent.click(screen.getByText('Retry'))

    await waitFor(() => {
      expect(screen.getByText('Main Bank')).toBeTruthy()
    })
    expect(fetch).toHaveBeenCalledTimes(2)
  })

  it('links to account detail and account creation routes', async () => {
    vi.mocked(fetch).mockResolvedValue(
      new Response(
        JSON.stringify([
          {
            id: 7,
            name: 'IBKR',
            account_type: 'broker',
            base_currency: 'EUR',
            summary_status: 'ok',
            total_amount: '1.00000000',
            total_currency: 'EUR',
          },
        ]),
        { status: 200, headers: { 'Content-Type': 'application/json' } }
      )
    )

    render(
      <MemoryRouter>
        <AccountsListPage />
      </MemoryRouter>
    )

    expect((await screen.findByRole('link', { name: /IBKR.*broker.*EUR.*View details/ })).getAttribute('href')).toBe(
      '/accounts/7'
    )
    expect(screen.getByRole('link', { name: 'Create account' }).getAttribute('href')).toBe(
      '/accounts/new'
    )
  })
})
