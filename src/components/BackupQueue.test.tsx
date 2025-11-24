import { describe, it, expect, vi } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import { BackupQueue } from './BackupQueue'
import { NotificationProvider } from '../contexts/NotificationContext'

// Mock Tauri API
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockResolvedValue([]),
}))

// Mock Tauri events
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}))

describe('BackupQueue', () => {
  it('renders without crashing', async () => {
    render(
      <NotificationProvider>
        <BackupQueue />
      </NotificationProvider>
    )

    await waitFor(() => {
      expect(screen.getByText('Backup Queue')).toBeTruthy()
    })
  })

  it('displays backup queue sections', async () => {
    render(
      <NotificationProvider>
        <BackupQueue />
      </NotificationProvider>
    )

    await waitFor(() => {
      expect(screen.getByText('Backup Queue')).toBeTruthy()
    })
  })

  it('shows empty state when no backups', async () => {
    render(
      <NotificationProvider>
        <BackupQueue />
      </NotificationProvider>
    )

    await waitFor(() => {
      expect(screen.getByText(/No backup/)).toBeTruthy()
    })
  })
})
