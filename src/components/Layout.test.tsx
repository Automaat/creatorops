import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import { Layout } from './Layout'
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

describe('Layout', () => {
  it('renders without crashing', () => {
    render(
      <NotificationProvider>
        <Layout />
      </NotificationProvider>
    )

    // Layout should render navigation
    expect(screen.getByText('Dashboard')).toBeTruthy()
    expect(screen.getByText('Import')).toBeTruthy()
    expect(screen.getByText('Projects')).toBeTruthy()
  })

  it('displays all navigation items', () => {
    render(
      <NotificationProvider>
        <Layout />
      </NotificationProvider>
    )

    expect(screen.getByText('Dashboard')).toBeTruthy()
    expect(screen.getByText('Import')).toBeTruthy()
    expect(screen.getByText('Projects')).toBeTruthy()
    expect(screen.getByText('Backup Queue')).toBeTruthy()
    expect(screen.getByText('Delivery')).toBeTruthy()
    expect(screen.getByText('History')).toBeTruthy()
    expect(screen.getByText('Settings')).toBeTruthy()
  })

  it('renders children content', () => {
    render(
      <NotificationProvider>
        <Layout>
          <div>Test Content</div>
        </Layout>
      </NotificationProvider>
    )

    expect(screen.getByText('Test Content')).toBeTruthy()
  })
})
