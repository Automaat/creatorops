import { beforeEach, describe, expect, it, vi } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import { userEvent } from '@testing-library/user-event'
import { History } from './History'
import { NotificationProvider } from '../contexts/NotificationContext'
import { invoke } from '@tauri-apps/api/core'

// Mock Tauri API
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}))

// Mock formatting utils
vi.mock('../utils/formatting', () => ({
  formatBytes: (bytes: number) => `${bytes} bytes`,
  formatDate: (date: string) => date,
}))

const mockImportHistory = [
  {
    completedAt: '2024-01-01T10:05:00Z',
    destinationPath: '/dest/path1',
    filesCopied: 10,
    filesSkipped: 2,
    id: '1',
    projectId: 'project-1',
    projectName: 'Test Project 1',
    sourcePath: '/source/path1',
    startedAt: '2024-01-01T10:00:00Z',
    status: 'success' as const,
    totalBytes: 1024,
  },
  {
    completedAt: '2024-01-02T10:10:00Z',
    destinationPath: '/dest/path2',
    filesCopied: 5,
    filesSkipped: 0,
    id: '2',
    projectId: 'project-2',
    projectName: 'Test Project 2',
    sourcePath: '/source/path2',
    startedAt: '2024-01-02T10:00:00Z',
    status: 'partial' as const,
    totalBytes: 2048,
  },
  {
    completedAt: '2024-01-03T10:15:00Z',
    destinationPath: '/dest/path3',
    errorMessage: 'Import failed due to permission error',
    filesCopied: 0,
    filesSkipped: 5,
    id: '3',
    projectId: 'project-3',
    projectName: 'Test Project 3',
    sourcePath: '/source/path3',
    startedAt: '2024-01-03T10:00:00Z',
    status: 'failed' as const,
    totalBytes: 0,
  },
]

const mockBackupHistory = [
  {
    completedAt: '2024-01-01T11:30:00Z',
    destinationName: 'External Drive',
    destinationPath: '/backup/drive1',
    filesCopied: 20,
    filesSkipped: 1,
    id: '1',
    projectId: 'project-1',
    projectName: 'Backup Project 1',
    startedAt: '2024-01-01T11:00:00Z',
    status: 'completed' as const,
    totalBytes: 4096,
  },
  {
    completedAt: '2024-01-02T11:45:00Z',
    destinationName: 'Cloud Storage',
    destinationPath: '/backup/cloud',
    errorMessage: 'Network connection lost',
    filesCopied: 15,
    filesSkipped: 0,
    id: '2',
    projectId: 'project-2',
    projectName: 'Backup Project 2',
    startedAt: '2024-01-02T11:00:00Z',
    status: 'failed' as const,
    totalBytes: 8192,
  },
  {
    completedAt: '2024-01-03T11:20:00Z',
    destinationName: 'NAS',
    destinationPath: '/backup/nas',
    filesCopied: 8,
    filesSkipped: 2,
    id: '3',
    projectId: 'project-3',
    projectName: 'Backup Project 3',
    startedAt: '2024-01-03T11:00:00Z',
    status: 'cancelled' as const,
    totalBytes: 3072,
  },
]

describe('history', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    vi.mocked(invoke).mockResolvedValue([])
  })

  it('renders without crashing', async () => {
    render(
      <NotificationProvider>
        <History />
      </NotificationProvider>
    )

    await waitFor(() => {
      expect(screen.getByText('History')).toBeTruthy()
    })
  })

  it('displays header and description', async () => {
    render(
      <NotificationProvider>
        <History />
      </NotificationProvider>
    )

    await waitFor(() => {
      expect(screen.getByText('History')).toBeTruthy()
      expect(screen.getByText('View import and backup history')).toBeTruthy()
    })
  })

  it('shows loading state initially', () => {
    render(
      <NotificationProvider>
        <History />
      </NotificationProvider>
    )

    expect(screen.getByText('Loading history...')).toBeTruthy()
  })

  it('shows empty state when no import history', async () => {
    render(
      <NotificationProvider>
        <History />
      </NotificationProvider>
    )

    await waitFor(() => {
      expect(screen.getByText('No import history yet')).toBeTruthy()
    })
  })

  it('displays import history list', async () => {
    vi.mocked(invoke).mockResolvedValue(mockImportHistory)

    render(
      <NotificationProvider>
        <History />
      </NotificationProvider>
    )

    await waitFor(() => {
      expect(screen.getByText('Test Project 1')).toBeTruthy()
      expect(screen.getByText('Test Project 2')).toBeTruthy()
      expect(screen.getByText('Test Project 3')).toBeTruthy()
    })
  })

  it('displays import history details', async () => {
    vi.mocked(invoke).mockResolvedValue(mockImportHistory)

    render(
      <NotificationProvider>
        <History />
      </NotificationProvider>
    )

    await waitFor(() => {
      expect(screen.getByText('/source/path1')).toBeTruthy()
      expect(screen.getByText('/dest/path1')).toBeTruthy()
      expect(screen.getByText('10')).toBeTruthy()
      expect(screen.getByText('1024 bytes')).toBeTruthy()
    })
  })

  it('displays status badges with correct styling for import history', async () => {
    vi.mocked(invoke).mockResolvedValue(mockImportHistory)

    render(
      <NotificationProvider>
        <History />
      </NotificationProvider>
    )

    await waitFor(() => {
      const successBadge = screen.getByText('success')
      expect(successBadge.className).toContain('status-success')

      const partialBadge = screen.getByText('partial')
      expect(partialBadge.className).toContain('status-warning')

      const failedBadge = screen.getByText('failed')
      expect(failedBadge.className).toContain('status-error')
    })
  })

  it('shows files skipped when greater than 0', async () => {
    vi.mocked(invoke).mockResolvedValue([mockImportHistory[0]])

    render(
      <NotificationProvider>
        <History />
      </NotificationProvider>
    )

    await waitFor(() => {
      expect(screen.getByText('Files Skipped')).toBeTruthy()
      expect(screen.getByText('2')).toBeTruthy()
    })
  })

  it('hides files skipped when 0', async () => {
    vi.mocked(invoke).mockResolvedValue([mockImportHistory[1]])

    render(
      <NotificationProvider>
        <History />
      </NotificationProvider>
    )

    await waitFor(() => {
      expect(screen.queryByText('Files Skipped')).toBeNull()
    })
  })

  it('displays error message when present', async () => {
    vi.mocked(invoke).mockResolvedValue([mockImportHistory[2]])

    render(
      <NotificationProvider>
        <History />
      </NotificationProvider>
    )

    await waitFor(() => {
      expect(screen.getByText('Import failed due to permission error')).toBeTruthy()
    })
  })

  it('switches to backup history tab', async () => {
    const user = userEvent.setup()
    vi.mocked(invoke).mockResolvedValue([])

    render(
      <NotificationProvider>
        <History />
      </NotificationProvider>
    )

    await waitFor(() => {
      expect(screen.getByText('No import history yet')).toBeTruthy()
    })

    const backupTab = screen.getByText(/Backups/)
    await user.click(backupTab)

    await waitFor(() => {
      expect(screen.getByText('No backup history yet')).toBeTruthy()
    })
  })

  it('displays backup history list', async () => {
    const user = userEvent.setup()
    vi.mocked(invoke).mockImplementation((cmd) => {
      if (cmd === 'get_backup_history') {
        return Promise.resolve(mockBackupHistory)
      }
      return Promise.resolve([])
    })

    render(
      <NotificationProvider>
        <History />
      </NotificationProvider>
    )

    const backupTab = screen.getByText(/Backups/)
    await user.click(backupTab)

    await waitFor(() => {
      expect(screen.getByText('Backup Project 1')).toBeTruthy()
      expect(screen.getByText('Backup Project 2')).toBeTruthy()
      expect(screen.getByText('Backup Project 3')).toBeTruthy()
    })
  })

  it('displays backup history details', async () => {
    const user = userEvent.setup()
    vi.mocked(invoke).mockImplementation((cmd) => {
      if (cmd === 'get_backup_history') {
        return Promise.resolve(mockBackupHistory)
      }
      return Promise.resolve([])
    })

    render(
      <NotificationProvider>
        <History />
      </NotificationProvider>
    )

    const backupTab = screen.getByText(/Backups/)
    await user.click(backupTab)

    await waitFor(() => {
      expect(screen.getByText(/External Drive/)).toBeTruthy()
      expect(screen.getByText(/\/backup\/drive1/)).toBeTruthy()
      expect(screen.getByText('20')).toBeTruthy()
      expect(screen.getByText('4096 bytes')).toBeTruthy()
    })
  })

  it('displays status badges with correct styling for backup history', async () => {
    const user = userEvent.setup()
    vi.mocked(invoke).mockImplementation((cmd) => {
      if (cmd === 'get_backup_history') {
        return Promise.resolve(mockBackupHistory)
      }
      return Promise.resolve([])
    })

    render(
      <NotificationProvider>
        <History />
      </NotificationProvider>
    )

    const backupTab = screen.getByText(/Backups/)
    await user.click(backupTab)

    await waitFor(() => {
      const completedBadge = screen.getByText('completed')
      expect(completedBadge.className).toContain('status-success')

      const failedBadge = screen.getByText('failed')
      expect(failedBadge.className).toContain('status-error')

      const cancelledBadge = screen.getByText('cancelled')
      expect(cancelledBadge.className).toContain('status-error')
    })
  })

  it('displays backup error message when present', async () => {
    const user = userEvent.setup()
    vi.mocked(invoke).mockImplementation((cmd) => {
      if (cmd === 'get_backup_history') {
        return Promise.resolve([mockBackupHistory[1]])
      }
      return Promise.resolve([])
    })

    render(
      <NotificationProvider>
        <History />
      </NotificationProvider>
    )

    const backupTab = screen.getByText(/Backups/)
    await user.click(backupTab)

    await waitFor(() => {
      expect(screen.getByText('Network connection lost')).toBeTruthy()
    })
  })

  it('shows correct tab counts', async () => {
    const user = userEvent.setup()
    vi.mocked(invoke).mockImplementation((cmd) => {
      if (cmd === 'get_import_history') {
        return Promise.resolve(mockImportHistory)
      }
      if (cmd === 'get_backup_history') {
        return Promise.resolve(mockBackupHistory)
      }
      return Promise.resolve([])
    })

    render(
      <NotificationProvider>
        <History />
      </NotificationProvider>
    )

    await waitFor(() => {
      expect(screen.getByText('Imports (3)')).toBeTruthy()
      expect(screen.getByText('Backups (0)')).toBeTruthy()
    })

    const backupTab = screen.getByText(/Backups/)
    await user.click(backupTab)

    await waitFor(() => {
      expect(screen.getByText('Backups (3)')).toBeTruthy()
    })
  })

  it('applies active tab styling', async () => {
    const user = userEvent.setup()
    vi.mocked(invoke).mockResolvedValue([])

    render(
      <NotificationProvider>
        <History />
      </NotificationProvider>
    )

    await waitFor(() => {
      const importTab = screen.getByText(/Imports/)
      expect(importTab.className).toContain('tab-active')
    })

    const backupTab = screen.getByText(/Backups/)
    await user.click(backupTab)

    await waitFor(() => {
      expect(backupTab.className).toContain('tab-active')
    })
  })

  it('invokes correct Tauri commands', async () => {
    const user = userEvent.setup()
    vi.mocked(invoke).mockResolvedValue([])

    render(
      <NotificationProvider>
        <History />
      </NotificationProvider>
    )

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('get_import_history', { limit: 100 })
    })

    const backupTab = screen.getByText(/Backups/)
    await user.click(backupTab)

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('get_backup_history')
    })
  })

  it('handles API error gracefully', async () => {
    const consoleError = vi.spyOn(console, 'error').mockImplementation(() => {})
    vi.mocked(invoke).mockRejectedValue(new Error('API error'))

    render(
      <NotificationProvider>
        <History />
      </NotificationProvider>
    )

    await waitFor(() => {
      expect(consoleError).toHaveBeenCalledWith('Failed to load history:', expect.any(Error))
    })

    consoleError.mockRestore()
  })

  it('reloads history when switching tabs', async () => {
    const user = userEvent.setup()
    vi.mocked(invoke).mockResolvedValue([])

    render(
      <NotificationProvider>
        <History />
      </NotificationProvider>
    )

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledTimes(1)
    })

    const backupTab = screen.getByText(/Backups/)
    await user.click(backupTab)

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledTimes(2)
    })

    const importTab = screen.getByText(/Imports/)
    await user.click(importTab)

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledTimes(3)
    })
  })
})
