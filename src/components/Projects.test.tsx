import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import { Projects } from './Projects'
import { NotificationProvider } from '../contexts/NotificationContext'
import type { Project } from '../types'
import { ProjectStatus } from '../types'
import { invoke } from '@tauri-apps/api/core'

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

const mockInvoke = vi.mocked(invoke)

// Helper to create mock projects
const createMockProject = (overrides?: Partial<Project>): Project => ({
  id: '1',
  name: 'Test Project',
  clientName: 'Test Client',
  date: '2024-01-15',
  shootType: 'Wedding',
  status: ProjectStatus.Editing,
  folderPath: '/path/to/project',
  createdAt: '2024-01-01T00:00:00Z',
  updatedAt: '2024-01-01T00:00:00Z',
  ...overrides,
})

describe('Projects', () => {
  beforeEach(() => {
    mockInvoke.mockResolvedValue([])
  })

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

  it('displays overdue badge when project deadline is past', async () => {
    const yesterday = new Date()
    yesterday.setDate(yesterday.getDate() - 1)
    const overdueProject = createMockProject({
      deadline: yesterday.toISOString().split('T')[0],
    })

    mockInvoke.mockResolvedValue([overdueProject])

    render(
      <NotificationProvider>
        <Projects />
      </NotificationProvider>
    )

    await waitFor(() => {
      expect(screen.getByText('Overdue')).toBeTruthy()
    })
  })

  it('does not display overdue badge when deadline is in future', async () => {
    const tomorrow = new Date()
    tomorrow.setDate(tomorrow.getDate() + 1)
    const futureProject = createMockProject({
      deadline: tomorrow.toISOString().split('T')[0],
    })

    mockInvoke.mockResolvedValue([futureProject])

    render(
      <NotificationProvider>
        <Projects />
      </NotificationProvider>
    )

    await waitFor(() => {
      expect(screen.getByText('Test Project')).toBeTruthy()
    })

    expect(screen.queryByText('Overdue')).toBeNull()
  })

  it('displays project status alongside overdue indicator', async () => {
    const overdueProject = createMockProject({
      deadline: '2020-01-01',
      status: ProjectStatus.Editing,
    })

    mockInvoke.mockResolvedValue([overdueProject])

    render(
      <NotificationProvider>
        <Projects />
      </NotificationProvider>
    )

    await waitFor(() => {
      expect(screen.getByText('Editing')).toBeTruthy()
      expect(screen.getByText('Overdue')).toBeTruthy()
    })
  })
})
