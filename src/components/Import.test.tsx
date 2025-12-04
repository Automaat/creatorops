import { beforeEach, describe, expect, it, vi } from 'vitest'
import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import { userEvent } from '@testing-library/user-event'
import { Import } from './Import'
import { NotificationProvider } from '../contexts/NotificationContext'
import type { CopyResult, Project, SDCard } from '../types'
import { ProjectStatus } from '../types'

// Mock Tauri API
vi.mock<typeof import('@tauri-apps/api/core')>('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}))

vi.mock<typeof import('@tauri-apps/api/event')>('@tauri-apps/api/event', () => ({
  listen: vi.fn(),
}))

vi.mock<typeof import('@tauri-apps/plugin-notification')>('@tauri-apps/plugin-notification', () => ({
  isPermissionGranted: vi.fn().mockResolvedValue(true), requestPermission: vi.fn().mockResolvedValue('granted'), sendNotification: vi.fn(),
}))

// Import the mocked modules to get references to the mock functions
const tauriCore = await import('@tauri-apps/api/core')
const tauriEvent = await import('@tauri-apps/api/event')
const mockInvoke = vi.mocked(tauriCore.invoke)
const mockListen = vi.mocked(tauriEvent.listen)

describe('import', () => {
  const mockSDCards: SDCard[] = [
    {
      deviceType: 'SD', fileCount: 100, freeSpace: 16_000_000_000, isRemovable: true, name: 'SD Card 1', path: '/Volumes/SDCARD1', size: 32_000_000_000,
    },
    {
      deviceType: 'SD', fileCount: 200, freeSpace: 32_000_000_000, isRemovable: true, name: 'SD Card 2', path: '/Volumes/SDCARD2', size: 64_000_000_000,
    },
  ]

  const mockProjects: Project[] = [
    {
      clientName: 'John & Jane', createdAt: '2024-01-15T10:00:00Z', date: '2024-01-15', folderPath: '/path/to/project1', id: 'project-1', name: 'Wedding Shoot', shootType: 'Wedding', status: ProjectStatus.New, updatedAt: '2024-01-15T10:00:00Z',
    },
    {
      clientName: 'Alice', createdAt: '2024-01-16T10:00:00Z', date: '2024-01-16', folderPath: '/path/to/project2', id: 'project-2', name: 'Portrait Session', shootType: 'Portrait', status: ProjectStatus.Editing, updatedAt: '2024-01-16T10:00:00Z',
    },
  ]

  beforeEach(() => {
    vi.clearAllMocks()
    mockInvoke.mockResolvedValue([])
    mockListen.mockResolvedValue(() => {})
    localStorage.clear()
  })

  const mockProps = {
    isScanning: false, onImportComplete: vi.fn(), sdCards: [],
  }

  it('renders without crashing', async () => {
    render(
      <NotificationProvider>
        <Import {...mockProps} />
      </NotificationProvider>
    )

    await waitFor(() => {
      expect(screen.getByRole('heading', { level: 1, name: 'Import from SD Card' })).toBeTruthy()
    })
  })

  it('shows scanning message when scanning', async () => {
    render(
      <NotificationProvider>
        <Import {...mockProps} isScanning />
      </NotificationProvider>
    )

    await waitFor(() => {
      expect(screen.getByText('Scanning for SD cards...')).toBeTruthy()
    })
  })

  it('shows empty state when no SD cards and not scanning', async () => {
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

  it('renders SD card list when cards are available', async () => {
    render(
      <NotificationProvider>
        <Import {...mockProps} sdCards={mockSDCards} />
      </NotificationProvider>
    )

    await waitFor(() => {
      expect(screen.getByText('SD Card 1')).toBeTruthy()
      expect(screen.getByText('SD Card 2')).toBeTruthy()
    })
  })

  it('shows collapsed card view by default', async () => {
    render(
      <NotificationProvider>
        <Import {...mockProps} sdCards={mockSDCards} />
      </NotificationProvider>
    )

    await waitFor(() => {
      expect(screen.getAllByText('Click to import')).toHaveLength(2)
    })
  })

  it('expands card when clicked and loads projects', async () => {
    mockInvoke.mockResolvedValue(mockProjects)
    const user = userEvent.setup()

    render(
      <NotificationProvider>
        <Import {...mockProps} sdCards={mockSDCards} />
      </NotificationProvider>
    )

    await waitFor(() => {
      expect(screen.getByText('SD Card 1')).toBeTruthy()
    })

    const card = screen.getAllByText('Click to import')[0].closest('.project-list-item')
    if (card) {await user.click(card)}

    await waitFor(() => {
      expect(screen.getByText('Select a project to import into')).toBeTruthy()
    })

    expect(mockInvoke).toHaveBeenCalledWith('list_projects')
  })

  it('opens project dropdown when expanded', async () => {
    mockInvoke.mockResolvedValue(mockProjects)
    const user = userEvent.setup()

    render(
      <NotificationProvider>
        <Import {...mockProps} sdCards={[mockSDCards[0]]} />
      </NotificationProvider>
    )

    const card = screen.getByText('Click to import').closest('.project-list-item')
    if (card) {await user.click(card)}

    await waitFor(() => {
      expect(screen.getByText('Choose a project...')).toBeTruthy()
    })

    const dropdownButton = screen.getByText('Choose a project...').closest('button')
    if (dropdownButton) {await user.click(dropdownButton)}

    await waitFor(() => {
      expect(screen.getByText('Wedding Shoot')).toBeTruthy()
      expect(screen.getByText('Portrait Session')).toBeTruthy()
    })
  })

  it('selects project from dropdown', async () => {
    mockInvoke.mockResolvedValue(mockProjects)
    const user = userEvent.setup()

    render(
      <NotificationProvider>
        <Import {...mockProps} sdCards={[mockSDCards[0]]} />
      </NotificationProvider>
    )

    const card = screen.getByText('Click to import').closest('.project-list-item')
    if (card) {await user.click(card)}

    await waitFor(() => {
      expect(screen.getByText('Choose a project...')).toBeTruthy()
    })

    const dropdownButton = screen.getByText('Choose a project...').closest('button')
    if (dropdownButton) {await user.click(dropdownButton)}

    await waitFor(() => {
      expect(screen.getByText('Wedding Shoot')).toBeTruthy()
    })

    await user.click(screen.getByText('Wedding Shoot'))

    await waitFor(() => {
      expect(screen.getByText('John & Jane · 2024-01-15 · Wedding')).toBeTruthy()
    })
  })

  it('shows create new project option in dropdown', async () => {
    mockInvoke.mockResolvedValue(mockProjects)
    const user = userEvent.setup()

    render(
      <NotificationProvider>
        <Import {...mockProps} sdCards={[mockSDCards[0]]} />
      </NotificationProvider>
    )

    const card = screen.getByText('Click to import').closest('.project-list-item')
    if (card) {await user.click(card)}

    await waitFor(() => {
      expect(screen.getByText('Choose a project...')).toBeTruthy()
    })

    const dropdownButton = screen.getByText('Choose a project...').closest('button')
    if (dropdownButton) {await user.click(dropdownButton)}

    await waitFor(() => {
      expect(screen.getByText('+ Create New Project')).toBeTruthy()
    })
  })

  it('opens create new project dialog when clicking create option', async () => {
    mockInvoke.mockResolvedValue(mockProjects)
    const user = userEvent.setup()

    render(
      <NotificationProvider>
        <Import {...mockProps} sdCards={[mockSDCards[0]]} />
      </NotificationProvider>
    )

    const card = screen.getByText('Click to import').closest('.project-list-item')
    if (card) {await user.click(card)}

    await waitFor(() => {
      expect(screen.getByText('Choose a project...')).toBeTruthy()
    })

    const dropdownButton = screen.getByText('Choose a project...').closest('button')
    if (dropdownButton) {await user.click(dropdownButton)}

    await waitFor(() => {
      expect(screen.getByText('+ Create New Project')).toBeTruthy()
    })

    await user.click(screen.getByText('+ Create New Project'))

    await waitFor(() => {
      expect(screen.getByText('Create New Project')).toBeTruthy()
    })
  })

  it('disables start import button when no project selected', async () => {
    mockInvoke.mockResolvedValue(mockProjects)
    const user = userEvent.setup()

    render(
      <NotificationProvider>
        <Import {...mockProps} sdCards={[mockSDCards[0]]} />
      </NotificationProvider>
    )

    const card = screen.getByText('Click to import').closest('.project-list-item')
    if (card) {await user.click(card)}

    await waitFor(() => {
      const startButton = screen.getByText('Start Import')
      expect(startButton).toBeTruthy()
      if (startButton instanceof HTMLButtonElement) {
        expect(startButton.disabled).toBe(true)
      }
    })
  })

  it('enables start import button after selecting project', async () => {
    mockInvoke.mockResolvedValue(mockProjects)
    const user = userEvent.setup()

    render(
      <NotificationProvider>
        <Import {...mockProps} sdCards={[mockSDCards[0]]} />
      </NotificationProvider>
    )

    const card = screen.getByText('Click to import').closest('.project-list-item')
    if (card) {await user.click(card)}

    await waitFor(() => {
      expect(screen.getByText('Choose a project...')).toBeTruthy()
    })

    const dropdownButton = screen.getByText('Choose a project...').closest('button')
    if (dropdownButton) {await user.click(dropdownButton)}

    await waitFor(() => {
      expect(screen.getByText('Wedding Shoot')).toBeTruthy()
    })

    await user.click(screen.getByText('Wedding Shoot'))

    await waitFor(() => {
      const startButton = screen.getByText('Start Import')
      if (startButton instanceof HTMLButtonElement) {
        expect(startButton.disabled).toBe(false)
      }
    })
  })

  it('shows success result after import completes', async () => {
    const mockCopyResult: CopyResult = {
      filesCopied: 10, filesSkipped: 0, photosCopied: 8, skippedFiles: [], success: true, totalBytes: 1_000_000, videosCopied: 2,
    }

    mockInvoke
      .mockResolvedValueOnce(mockProjects)
      .mockResolvedValueOnce()
      .mockResolvedValueOnce(['file1.jpg', 'file2.jpg'])
      .mockResolvedValueOnce(mockCopyResult)
      .mockResolvedValueOnce()

    const user = userEvent.setup()

    render(
      <NotificationProvider>
        <Import {...mockProps} sdCards={[mockSDCards[0]]} />
      </NotificationProvider>
    )

    const card = screen.getByText('Click to import').closest('.project-list-item')
    if (card) {await user.click(card)}

    await waitFor(() => {
      expect(screen.getByText('Choose a project...')).toBeTruthy()
    })

    const dropdownButton = screen.getByText('Choose a project...').closest('button')
    if (dropdownButton) {await user.click(dropdownButton)}
    await user.click(screen.getByText('Wedding Shoot'))
    await user.click(screen.getByText('Start Import'))

    await waitFor(() => {
      expect(screen.getByText('Import completed')).toBeTruthy()
      expect(screen.getByText('Files copied: 10')).toBeTruthy()
      expect(screen.getByText('Photos: 8')).toBeTruthy()
      expect(screen.getByText('Videos: 2')).toBeTruthy()
    })
  })

  it('shows failure result when import fails', async () => {
    mockInvoke
      .mockResolvedValueOnce(mockProjects)
      .mockResolvedValueOnce()
      .mockResolvedValueOnce(['file1.jpg'])
      .mockRejectedValueOnce(new Error('Import failed'))
      .mockResolvedValueOnce()

    const user = userEvent.setup()

    render(
      <NotificationProvider>
        <Import {...mockProps} sdCards={[mockSDCards[0]]} />
      </NotificationProvider>
    )

    const card = screen.getByText('Click to import').closest('.project-list-item')
    if (card) {await user.click(card)}

    await waitFor(() => {
      expect(screen.getByText('Choose a project...')).toBeTruthy()
    })

    const dropdownButton = screen.getByText('Choose a project...').closest('button')
    if (dropdownButton) {await user.click(dropdownButton)}
    await user.click(screen.getByText('Wedding Shoot'))
    await user.click(screen.getByText('Start Import'))

    await waitFor(() => {
      expect(screen.getByText('Import failed')).toBeTruthy()
    })
  })

  it('shows skipped files when some files are skipped', async () => {
    const mockCopyResult: CopyResult = {
      filesCopied: 8, filesSkipped: 2, photosCopied: 6, skippedFiles: ['duplicate1.jpg', 'duplicate2.jpg'], success: true, totalBytes: 800_000, videosCopied: 2,
    }

    mockInvoke
      .mockResolvedValueOnce(mockProjects)
      .mockResolvedValueOnce()
      .mockResolvedValueOnce(['file1.jpg', 'file2.jpg'])
      .mockResolvedValueOnce(mockCopyResult)
      .mockResolvedValueOnce()

    const user = userEvent.setup()

    render(
      <NotificationProvider>
        <Import {...mockProps} sdCards={[mockSDCards[0]]} />
      </NotificationProvider>
    )

    const card = screen.getByText('Click to import').closest('.project-list-item')
    if (card) {await user.click(card)}

    await waitFor(() => {
      expect(screen.getByText('Choose a project...')).toBeTruthy()
    })

    const dropdownButton = screen.getByText('Choose a project...').closest('button')
    if (dropdownButton) {await user.click(dropdownButton)}
    await user.click(screen.getByText('Wedding Shoot'))
    await user.click(screen.getByText('Start Import'))

    await waitFor(() => {
      expect(screen.getByText('Files skipped: 2')).toBeTruthy()
      expect(screen.getByText('duplicate1.jpg')).toBeTruthy()
      expect(screen.getByText('duplicate2.jpg')).toBeTruthy()
    })
  })

  it('handles no files found on SD card', async () => {
    mockInvoke
      .mockResolvedValueOnce(mockProjects)
      .mockResolvedValueOnce()
      .mockResolvedValueOnce([])
      .mockResolvedValueOnce()

    const user = userEvent.setup()

    render(
      <NotificationProvider>
        <Import {...mockProps} sdCards={[mockSDCards[0]]} />
      </NotificationProvider>
    )

    const card = screen.getByText('Click to import').closest('.project-list-item')
    if (card) {await user.click(card)}

    await waitFor(() => {
      expect(screen.getByText('Choose a project...')).toBeTruthy()
    })

    const dropdownButton = screen.getByText('Choose a project...').closest('button')
    if (dropdownButton) {await user.click(dropdownButton)}
    await user.click(screen.getByText('Wedding Shoot'))
    await user.click(screen.getByText('Start Import'))

    await waitFor(() => {
      expect(screen.getByText('No photo or video files found on SD card')).toBeTruthy()
    })
  })

  it('closes dropdown when clicking outside', async () => {
    mockInvoke.mockResolvedValue(mockProjects)
    const user = userEvent.setup()

    render(
      <NotificationProvider>
        <Import {...mockProps} sdCards={[mockSDCards[0]]} />
      </NotificationProvider>
    )

    const card = screen.getByText('Click to import').closest('.project-list-item')
    if (card) {await user.click(card)}

    await waitFor(() => {
      expect(screen.getByText('Choose a project...')).toBeTruthy()
    })

    const dropdownButton = screen.getByText('Choose a project...').closest('button')
    if (dropdownButton) {await user.click(dropdownButton)}

    await waitFor(() => {
      expect(screen.getByText('Wedding Shoot')).toBeTruthy()
    })

    fireEvent.mouseDown(document.body)

    await waitFor(() => {
      expect(screen.queryByText('Portrait Session')).toBeNull()
    })
  })

  it('resets active card when card is removed from list', async () => {
    const { rerender } = render(
      <NotificationProvider>
        <Import {...mockProps} sdCards={mockSDCards} />
      </NotificationProvider>
    )

    await waitFor(() => {
      expect(screen.getByText('SD Card 1')).toBeTruthy()
    })

    rerender(
      <NotificationProvider>
        <Import {...mockProps} sdCards={[mockSDCards[1]]} />
      </NotificationProvider>
    )

    await waitFor(() => {
      expect(screen.queryByText('SD Card 1')).toBeNull()
      expect(screen.getByText('SD Card 2')).toBeTruthy()
    })
  })

  it('shows create new project button when no projects available', async () => {
    mockInvoke.mockResolvedValue([])
    const user = userEvent.setup()

    render(
      <NotificationProvider>
        <Import {...mockProps} sdCards={[mockSDCards[0]]} />
      </NotificationProvider>
    )

    const card = screen.getByText('Click to import').closest('.project-list-item')
    if (card) {await user.click(card)}

    await waitFor(() => {
      expect(screen.getByText('Choose a project...')).toBeTruthy()
    })

    const dropdownButton = screen.getByText('Choose a project...').closest('button')
    if (dropdownButton) {await user.click(dropdownButton)}

    await waitFor(() => {
      expect(screen.getByText('No projects available')).toBeTruthy()
      expect(screen.getByRole('button', { name: 'Create New Project' })).toBeTruthy()
    })
  })

  it('shows cancelled message when import is cancelled', async () => {
    const mockCopyResult: CopyResult = {
      error: 'Import cancelled by user', filesCopied: 5, filesSkipped: 0, photosCopied: 4, skippedFiles: [], success: false, totalBytes: 500_000, videosCopied: 1,
    }

    mockInvoke
      .mockResolvedValueOnce(mockProjects)
      .mockResolvedValueOnce()
      .mockResolvedValueOnce(['file1.jpg'])
      .mockResolvedValueOnce(mockCopyResult)

    const user = userEvent.setup()

    render(
      <NotificationProvider>
        <Import {...mockProps} sdCards={[mockSDCards[0]]} />
      </NotificationProvider>
    )

    const card = screen.getByText('Click to import').closest('.project-list-item')
    if (card) {await user.click(card)}

    await waitFor(() => {
      expect(screen.getByText('Choose a project...')).toBeTruthy()
    })

    const dropdownButton = screen.getByText('Choose a project...').closest('button')
    if (dropdownButton) {await user.click(dropdownButton)}
    await user.click(screen.getByText('Wedding Shoot'))
    await user.click(screen.getByText('Start Import'))

    await waitFor(() => {
      expect(screen.getByText('Import cancelled (5 files copied)')).toBeTruthy()
    })
  })

  it('closes result view when done button is clicked', async () => {
    const mockCopyResult: CopyResult = {
      filesCopied: 10, filesSkipped: 0, photosCopied: 8, skippedFiles: [], success: true, totalBytes: 1_000_000, videosCopied: 2,
    }

    mockInvoke
      .mockResolvedValueOnce(mockProjects)
      .mockResolvedValueOnce()
      .mockResolvedValueOnce(['file1.jpg'])
      .mockResolvedValueOnce(mockCopyResult)
      .mockResolvedValueOnce()

    const user = userEvent.setup()

    render(
      <NotificationProvider>
        <Import {...mockProps} sdCards={[mockSDCards[0]]} />
      </NotificationProvider>
    )

    const card = screen.getByText('Click to import').closest('.project-list-item')
    if (card) {await user.click(card)}

    await waitFor(() => {
      expect(screen.getByText('Choose a project...')).toBeTruthy()
    })

    const dropdownButton = screen.getByText('Choose a project...').closest('button')
    if (dropdownButton) {await user.click(dropdownButton)}
    await user.click(screen.getByText('Wedding Shoot'))
    await user.click(screen.getByText('Start Import'))

    await waitFor(() => {
      expect(screen.getByText('Import completed')).toBeTruthy()
    })

    await user.click(screen.getByText('Done'))

    await waitFor(() => {
      expect(screen.getByText('Select a project to import into')).toBeTruthy()
    })
  })

  it('calls updateProjectStatus when import starts', async () => {
    const mockCopyResult: CopyResult = {
      filesCopied: 10, filesSkipped: 0, photosCopied: 8, skippedFiles: [], success: true, totalBytes: 1_000_000, videosCopied: 2,
    }

    mockInvoke
      .mockResolvedValueOnce(mockProjects)
      .mockResolvedValueOnce()
      .mockResolvedValueOnce(['file1.jpg'])
      .mockResolvedValueOnce(mockCopyResult)
      .mockResolvedValueOnce()

    const user = userEvent.setup()

    render(
      <NotificationProvider>
        <Import {...mockProps} sdCards={[mockSDCards[0]]} />
      </NotificationProvider>
    )

    const card = screen.getByText('Click to import').closest('.project-list-item')
    if (card) {await user.click(card)}

    await waitFor(() => {
      expect(screen.getByText('Choose a project...')).toBeTruthy()
    })

    const dropdownButton = screen.getByText('Choose a project...').closest('button')
    if (dropdownButton) {await user.click(dropdownButton)}
    await user.click(screen.getByText('Wedding Shoot'))
    await user.click(screen.getByText('Start Import'))

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('update_project_status', {
        newStatus: ProjectStatus.Importing, projectId: 'project-1',
      })
    })
  })

  it('calls save_import_history after import completes', async () => {
    const mockCopyResult: CopyResult = {
      filesCopied: 10, filesSkipped: 0, photosCopied: 8, skippedFiles: [], success: true, totalBytes: 1_000_000, videosCopied: 2,
    }

    mockInvoke
      .mockResolvedValueOnce(mockProjects)
      .mockResolvedValueOnce()
      .mockResolvedValueOnce(['file1.jpg'])
      .mockResolvedValueOnce(mockCopyResult)
      .mockResolvedValueOnce()

    const user = userEvent.setup()

    render(
      <NotificationProvider>
        <Import {...mockProps} sdCards={[mockSDCards[0]]} />
      </NotificationProvider>
    )

    const card = screen.getByText('Click to import').closest('.project-list-item')
    if (card) {await user.click(card)}

    await waitFor(() => {
      expect(screen.getByText('Choose a project...')).toBeTruthy()
    })

    const dropdownButton = screen.getByText('Choose a project...').closest('button')
    if (dropdownButton) {await user.click(dropdownButton)}
    await user.click(screen.getByText('Wedding Shoot'))
    await user.click(screen.getByText('Start Import'))

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith(
        'save_import_history',
        expect.objectContaining({
          filesCopied: 10, projectId: 'project-1',
        })
      )
    })
  })
})
