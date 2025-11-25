import { describe, it, expect, vi } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import { Delivery } from './Delivery'
import { NotificationProvider } from '../contexts/NotificationContext'

// Mock Tauri API
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockResolvedValue([]),
}))

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}))

vi.mock('@tauri-apps/plugin-dialog', () => ({
  open: vi.fn(),
}))

describe('Delivery', () => {
  it('renders without crashing', async () => {
    render(
      <NotificationProvider>
        <Delivery />
      </NotificationProvider>
    )

    await waitFor(() => {
      expect(screen.getByRole('heading', { name: 'Client Delivery', level: 1 })).toBeTruthy()
    })
  })

  it('displays delivery section', async () => {
    render(
      <NotificationProvider>
        <Delivery />
      </NotificationProvider>
    )

    await waitFor(() => {
      expect(screen.getByRole('heading', { name: 'Client Delivery', level: 1 })).toBeTruthy()
    })
  })

  it('shows empty state when no deliveries', async () => {
    render(
      <NotificationProvider>
        <Delivery />
      </NotificationProvider>
    )

    await waitFor(() => {
      expect(screen.getByText('No deliveries yet')).toBeTruthy()
    })
  })
})
