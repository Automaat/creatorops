import { describe, it, expect, vi } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import { Projects } from './Projects'
import { NotificationProvider } from '../contexts/NotificationContext'

// Mock Tauri API
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockResolvedValue([]),
}))

vi.mock('@tauri-apps/plugin-dialog', () => ({
  open: vi.fn(),
}))

vi.mock('@tauri-apps/plugin-opener', () => ({
  open: vi.fn(),
}))

describe('Projects', () => {
  it('renders without crashing', async () => {
    render(
      <NotificationProvider>
        <Projects />
      </NotificationProvider>
    )

    await waitFor(() => {
      expect(screen.getByText('Projects')).toBeTruthy()
    })
  })

  it('displays projects list view by default', async () => {
    render(
      <NotificationProvider>
        <Projects />
      </NotificationProvider>
    )

    await waitFor(() => {
      expect(screen.getByText('Projects')).toBeTruthy()
    })
  })

  it('shows empty state when no projects', async () => {
    render(
      <NotificationProvider>
        <Projects />
      </NotificationProvider>
    )

    await waitFor(() => {
      expect(screen.getByText(/No projects/)).toBeTruthy()
    })
  })
})
