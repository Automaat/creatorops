import { describe, it, expect, vi } from 'vitest'
import { render } from '@testing-library/react'
import App from './App'
import { NotificationProvider } from './contexts/NotificationContext'

// Mock Tauri APIs
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockResolvedValue([]),
}))

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}))

vi.mock('@tauri-apps/plugin-notification', () => ({
  sendNotification: vi.fn(),
  isPermissionGranted: vi.fn().mockResolvedValue(true),
  requestPermission: vi.fn().mockResolvedValue('granted'),
}))

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
