import { describe, it, expect, vi } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import { Import } from './Import'
import { NotificationProvider } from '../contexts/NotificationContext'

// Mock Tauri API
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockResolvedValue([]),
}))

vi.mock('@tauri-apps/plugin-notification', () => ({
  sendNotification: vi.fn(),
  isPermissionGranted: vi.fn().mockResolvedValue(true),
  requestPermission: vi.fn().mockResolvedValue('granted'),
}))

describe('Import', () => {
  const mockProps = {
    sdCards: [],
    isScanning: false,
    onImportComplete: vi.fn(),
  }

  it('renders without crashing', async () => {
    render(
      <NotificationProvider>
        <Import {...mockProps} />
      </NotificationProvider>
    )

    await waitFor(() => {
      expect(screen.getByRole('heading', { name: 'Import from SD Card', level: 1 })).toBeTruthy()
    })
  })

  it('displays import section', async () => {
    render(
      <NotificationProvider>
        <Import {...mockProps} />
      </NotificationProvider>
    )

    await waitFor(() => {
      expect(screen.getByRole('heading', { name: 'Import from SD Card', level: 1 })).toBeTruthy()
    })
  })

  it('shows empty state when no SD cards', async () => {
    render(
      <NotificationProvider>
        <Import {...mockProps} />
      </NotificationProvider>
    )

    await waitFor(() => {
      expect(
        screen.getByText('No SD cards detected. Insert an SD card and click Refresh.')
      ).toBeTruthy()
    })
  })
})
