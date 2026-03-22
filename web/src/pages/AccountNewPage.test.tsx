import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { MemoryRouter, Route, Routes } from 'react-router-dom'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import { AccountNewPage } from './AccountNewPage'

describe('AccountNewPage', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn())
  })

  afterEach(() => {
    cleanup()
    vi.unstubAllGlobals()
    vi.restoreAllMocks()
  })

  it('renders the account creation form', () => {
    render(
      <MemoryRouter>
        <AccountNewPage />
      </MemoryRouter>
    )

    expect(screen.getByText('New Account')).toBeTruthy()
    expect(screen.getByLabelText('Name')).toBeTruthy()
    expect(screen.getByLabelText('Account type')).toBeTruthy()
    expect(screen.getByLabelText('Base currency')).toBeTruthy()
  })

  it('creates an account and returns to the accounts list route', async () => {
    vi.mocked(fetch).mockResolvedValue(
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

    render(
      <MemoryRouter initialEntries={['/accounts/new']}>
        <Routes>
          <Route path="/accounts/new" element={<AccountNewPage />} />
          <Route path="/accounts" element={<div>Accounts Route</div>} />
        </Routes>
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

    expect(await screen.findByText('Accounts Route')).toBeTruthy()
    expect(fetch).toHaveBeenCalledWith(
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
      <MemoryRouter>
        <AccountNewPage />
      </MemoryRouter>
    )

    fireEvent.change(screen.getByLabelText('Name'), {
      target: { value: 'IBKR' },
    })
    fireEvent.click(screen.getByRole('button', { name: 'Create account' }))

    expect(await screen.findByText('Invalid currency format')).toBeTruthy()
  })
})
