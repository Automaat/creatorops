import { describe, it, expect, vi } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import { History } from './History'
import { NotificationProvider } from '../contexts/NotificationContext'

// Mock Tauri API
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockResolvedValue([]),
}))

describe('History', () => {
  it('renders without crashing', async () => {
    render(
      <NotificationProvider>
        <History />
      </NotificationProvider>
    )

    await waitFor(() => {
      expect(screen.getByText('History')).toBeTruthy()
    })
  })

  it('displays history section', async () => {
    render(
      <NotificationProvider>
        <History />
      </NotificationProvider>
    )

    await waitFor(() => {
      expect(screen.getByText('History')).toBeTruthy()
    })
  })

  it('shows empty state when no history', async () => {
    render(
      <NotificationProvider>
        <History />
      </NotificationProvider>
    )

    await waitFor(() => {
      expect(screen.getByText(/No history|No items|No activities/)).toBeTruthy()
    })
  })
})
