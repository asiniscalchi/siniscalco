import { renderToStaticMarkup } from 'react-dom/server'
import { MemoryRouter } from 'react-router-dom'
import { describe, expect, it } from 'vitest'

import App from './App'

describe('App', () => {
  it('renders the accounts list skeleton', () => {
    const html = renderToStaticMarkup(
      <MemoryRouter initialEntries={['/accounts']}>
        <App />
      </MemoryRouter>
    )

    expect(html).toContain('Accounts')
    expect(html).toContain('Create account')
    expect(html).toContain('Cash Accounts')
  })
})
