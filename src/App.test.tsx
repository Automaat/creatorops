import { describe, expect, it, vi } from 'vitest'
import { render } from '@testing-library/react'
import App from './App'
import { NotificationProvider } from './contexts/NotificationContext'

// Mock Tauri APIs
vi.mock<typeof import('@tauri-apps/api/core')>('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockResolvedValue([]),
}))

vi.mock<typeof import('@tauri-apps/api/event')>('@tauri-apps/api/event', () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}))

vi.mock<typeof import('@tauri-apps/plugin-notification')>('@tauri-apps/plugin-notification', () => ({
  isPermissionGranted: vi.fn().mockResolvedValue(true), requestPermission: vi.fn().mockResolvedValue('granted'), sendNotification: vi.fn(),
}))

describe('app', () => {
  it('renders without crashing', () => {
    const { container } = render(
      <NotificationProvider>
        <App />
      </NotificationProvider>
    )
    expect(container).toBeTruthy()
  })
})
