import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import { MemoryRouter } from 'react-router-dom'
import { describe, expect, it } from 'vitest'
import { afterEach, beforeEach, vi } from 'vitest'

import App from './App'

describe('App', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  afterEach(() => {
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
})
