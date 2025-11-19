import { describe, it, expect } from 'vitest'
import { render } from '@testing-library/react'
import App from './App'
import { NotificationProvider } from './contexts/NotificationContext'

describe('App', () => {
  it('renders without crashing', () => {
    const { container } = render(
      <NotificationProvider>
        <App />
      </NotificationProvider>
    )
    expect(container).toBeTruthy()
  })
})
