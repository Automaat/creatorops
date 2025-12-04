import { describe, expect, it, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import { Layout } from './Layout'
import { NotificationProvider } from '../contexts/NotificationContext'

// Mock Tauri API
vi.mock<typeof import('@tauri-apps/api/core')>('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockResolvedValue([]),
}))

vi.mock<typeof import('@tauri-apps/plugin-notification')>('@tauri-apps/plugin-notification', () => ({
  isPermissionGranted: vi.fn().mockResolvedValue(true), requestPermission: vi.fn().mockResolvedValue('granted'), sendNotification: vi.fn(),
}))

describe('layout', () => {
  it('renders without crashing', () => {
    render(
      <NotificationProvider>
        <Layout currentView="dashboard" onNavigate={vi.fn()}>
          <div />
        </Layout>
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
        <Layout currentView="dashboard" onNavigate={vi.fn()}>
          <div />
        </Layout>
      </NotificationProvider>
    )

    expect(screen.getByText('Dashboard')).toBeTruthy()
    expect(screen.getByText('Import')).toBeTruthy()
    expect(screen.getByText('Projects')).toBeTruthy()
    expect(screen.getByText('Backup Queue')).toBeTruthy()
    expect(screen.getByText('Delivery')).toBeTruthy()
    // "History" appears twice: as section title and nav item
    expect(screen.getAllByText('History')).toHaveLength(2)
    // "Settings" appears twice: as section title and nav item
    expect(screen.getAllByText('Settings')).toHaveLength(2)
  })

  it('renders children content', () => {
    render(
      <NotificationProvider>
        <Layout currentView="dashboard" onNavigate={vi.fn()}>
          <div>Test Content</div>
        </Layout>
      </NotificationProvider>
    )

    expect(screen.getByText('Test Content')).toBeTruthy()
  })
})
