import { describe, expect, it, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import { Settings } from './Settings'
import { NotificationProvider } from '../contexts/NotificationContext'

// Mock Tauri API
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockResolvedValue([]),
}))

vi.mock('@tauri-apps/plugin-dialog', () => ({
  open: vi.fn(),
}))

describe('settings', () => {
  it('renders without crashing', () => {
    render(
      <NotificationProvider>
        <Settings />
      </NotificationProvider>
    )

    expect(screen.getByText('Settings')).toBeTruthy()
  })

  it('displays appearance section', () => {
    render(
      <NotificationProvider>
        <Settings />
      </NotificationProvider>
    )

    expect(screen.getByText('Appearance')).toBeTruthy()
    expect(screen.getByText(/Theme/)).toBeTruthy()
  })

  it('displays backup destinations section', () => {
    render(
      <NotificationProvider>
        <Settings />
      </NotificationProvider>
    )

    expect(screen.getByText('Backup Destinations')).toBeTruthy()
  })
})
