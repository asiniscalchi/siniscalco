import { renderToStaticMarkup } from 'react-dom/server'
import { describe, expect, it } from 'vitest'

import App from './App'

describe('App', () => {
  it('renders the homepage content', () => {
    const html = renderToStaticMarkup(<App />)

    expect(html).toContain('siniscalco')
    expect(html).toContain('Hello World!')
    expect(html).toContain('Minimal homepage built with shadcn components.')
  })
})
