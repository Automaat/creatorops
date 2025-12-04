import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, waitFor, act } from '@testing-library/react'
import { userEvent } from '@testing-library/user-event'
import { BackupQueue } from './BackupQueue'
import { NotificationProvider } from '../contexts/NotificationContext'
import type { BackupJob, BackupProgress } from '../types'

const mockInvoke = vi.fn()
const mockListen = vi.fn()

// Mock Tauri API
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}))

// Mock Tauri events
vi.mock('@tauri-apps/api/event', () => ({
  listen: (...args: unknown[]) => mockListen(...args),
}))

describe('BackupQueue', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockInvoke.mockResolvedValue([])
    mockListen.mockImplementation(() => {
      return Promise.resolve(() => {})
    })
  })

  const renderComponent = () =>
    render(
      <NotificationProvider>
        <BackupQueue />
      </NotificationProvider>
    )

  const createMockJob = (overrides: Partial<BackupJob> = {}): BackupJob => ({
    id: 'job-1',
    projectId: 'proj-1',
    projectName: 'Test Project',
    sourcePath: '/source',
    destinationId: 'dest-1',
    destinationName: 'Backup Drive',
    destinationPath: '/backup',
    status: 'pending',
    totalFiles: 100,
    filesCopied: 0,
    filesSkipped: 0,
    totalBytes: 1000000,
    bytesTransferred: 0,
    createdAt: '2024-01-01T00:00:00Z',
    ...overrides,
  })

  const createMockProgress = (overrides: Partial<BackupProgress> = {}): BackupProgress => ({
    jobId: 'job-1',
    fileName: 'test.jpg',
    currentFile: 50,
    totalFiles: 100,
    bytesTransferred: 500000,
    totalBytes: 1000000,
    speed: 1000000,
    eta: 30,
    ...overrides,
  })

  describe('Empty state', () => {
    it('shows empty state when no jobs', async () => {
      mockInvoke.mockResolvedValue([])
      renderComponent()

      await waitFor(() => {
        expect(screen.getByText('No backup jobs in queue')).toBeTruthy()
        expect(screen.getByText('Create a backup from the Projects view')).toBeTruthy()
      })
    })
  })

  describe('Pending jobs', () => {
    it('displays pending jobs section', async () => {
      const pendingJob = createMockJob({ status: 'pending' })
      mockInvoke.mockResolvedValue([pendingJob])
      renderComponent()

      await waitFor(() => {
        expect(screen.getByRole('heading', { name: 'Pending' })).toBeTruthy()
        expect(screen.getByText('Test Project')).toBeTruthy()
        expect(screen.getByText('→ Backup Drive')).toBeTruthy()
        expect(screen.getByText('100 files')).toBeTruthy()
      })
    })

    it('displays job metadata for pending jobs', async () => {
      const pendingJob = createMockJob({
        status: 'pending',
        totalFiles: 150,
        totalBytes: 2000000,
      })
      mockInvoke.mockResolvedValue([pendingJob])
      renderComponent()

      await waitFor(() => {
        expect(screen.getByText('150 files')).toBeTruthy()
        expect(screen.getByText(/1\.91 MB|2\.0 MB/)).toBeTruthy()
      })
    })

    it('shows start backup button for pending jobs', async () => {
      const pendingJob = createMockJob({ status: 'pending' })
      mockInvoke.mockResolvedValue([pendingJob])
      renderComponent()

      await waitFor(() => {
        expect(screen.getByText('Start Backup')).toBeTruthy()
      })
    })

    it('shows cancel button for pending jobs', async () => {
      const pendingJob = createMockJob({ status: 'pending' })
      mockInvoke.mockResolvedValue([pendingJob])
      renderComponent()

      await waitFor(() => {
        expect(screen.getByText('Cancel')).toBeTruthy()
      })
    })

    it('displays multiple pending jobs', async () => {
      const job1 = createMockJob({ id: 'job-1', projectName: 'Project 1', status: 'pending' })
      const job2 = createMockJob({ id: 'job-2', projectName: 'Project 2', status: 'pending' })
      mockInvoke.mockResolvedValue([job1, job2])
      renderComponent()

      await waitFor(() => {
        expect(screen.getByText('Project 1')).toBeTruthy()
        expect(screen.getByText('Project 2')).toBeTruthy()
      })
    })
  })

  describe('Active jobs', () => {
    it('displays active jobs section', async () => {
      const activeJob = createMockJob({ status: 'inprogress' })
      mockInvoke.mockResolvedValue([activeJob])
      renderComponent()

      await waitFor(() => {
        expect(screen.getByRole('heading', { name: 'In Progress' })).toBeTruthy()
        expect(screen.getByText('Test Project')).toBeTruthy()
      })
    })

    it('displays progress bar for active jobs without progress data', async () => {
      const activeJob = createMockJob({
        status: 'inprogress',
        totalBytes: 1000000,
        bytesTransferred: 500000,
      })
      mockInvoke.mockResolvedValue([activeJob])
      renderComponent()

      await waitFor(() => {
        expect(screen.getByText('Test Project')).toBeTruthy()
      })
    })

    it('displays progress information when progress data available', async () => {
      const activeJob = createMockJob({
        status: 'inprogress',
        totalBytes: 1000000,
        bytesTransferred: 500000,
      })
      mockInvoke.mockResolvedValue([activeJob])

      let progressCallback: ((event: { payload: BackupProgress }) => void) | null = null
      mockListen.mockImplementation((event: string, callback: (e: unknown) => void) => {
        if (event === 'backup-progress') {
          progressCallback = callback as (event: { payload: BackupProgress }) => void
        }
        return Promise.resolve(() => {})
      })

      renderComponent()

      await waitFor(() => {
        expect(screen.getByText('Test Project')).toBeTruthy()
      })

      const progress = createMockProgress()
      if (progressCallback) {
        await act(async () => {
          progressCallback!({ payload: progress })
        })
      }

      await waitFor(() => {
        expect(screen.getByText('test.jpg')).toBeTruthy()
        expect(screen.getByText('50 / 100 files')).toBeTruthy()
        expect(screen.getByText(/488\.28 KB|500\.0 kB/, { exact: false })).toBeTruthy()
        expect(screen.getByText(/976\.56 KB\/s|1\.0 MB\/s/, { exact: false })).toBeTruthy()
        expect(screen.getByText(/ETA:/, { exact: false })).toBeTruthy()
      })
    })

    it('calculates progress percentage correctly', async () => {
      const activeJob = createMockJob({
        status: 'inprogress',
        totalBytes: 1000,
        bytesTransferred: 500,
      })
      mockInvoke.mockResolvedValue([activeJob])

      let progressCallback: ((event: { payload: BackupProgress }) => void) | null = null
      mockListen.mockImplementation((event: string, callback: (e: unknown) => void) => {
        if (event === 'backup-progress') {
          progressCallback = callback as (event: { payload: BackupProgress }) => void
        }
        return Promise.resolve(() => {})
      })

      renderComponent()

      await waitFor(() => {
        expect(screen.getByText('Test Project')).toBeTruthy()
      })

      const progress = createMockProgress()
      if (progressCallback) {
        await act(async () => {
          progressCallback!({ payload: progress })
        })
      }

      await waitFor(() => {
        const progressBar = document.querySelector('.progress-fill')
        expect(progressBar).toBeTruthy()
        expect((progressBar as HTMLElement).style.width).toBe('50%')
      })
    })
  })

  describe('Completed jobs', () => {
    it('displays completed jobs section', async () => {
      const completedJob = createMockJob({
        status: 'completed',
        filesCopied: 100,
        bytesTransferred: 1000000,
      })
      mockInvoke.mockResolvedValue([completedJob])
      renderComponent()

      await waitFor(() => {
        expect(screen.getByRole('heading', { name: 'Completed' })).toBeTruthy()
        expect(screen.getByText('Test Project')).toBeTruthy()
      })
    })

    it('displays completed status', async () => {
      const completedJob = createMockJob({ status: 'completed', filesCopied: 100 })
      mockInvoke.mockResolvedValue([completedJob])
      renderComponent()

      await waitFor(() => {
        expect(screen.getByRole('heading', { name: 'Completed' })).toBeTruthy()
        expect(screen.getByText('100 files copied')).toBeTruthy()
      })
    })

    it('displays failed status', async () => {
      const failedJob = createMockJob({
        status: 'failed',
        errorMessage: 'Disk full',
      })
      mockInvoke.mockResolvedValue([failedJob])
      renderComponent()

      await waitFor(() => {
        expect(screen.getByRole('heading', { name: 'Completed' })).toBeTruthy()
        expect(screen.getByText('Disk full')).toBeTruthy()
      })
    })

    it('displays cancelled status', async () => {
      const cancelledJob = createMockJob({ status: 'cancelled' })
      mockInvoke.mockResolvedValue([cancelledJob])
      renderComponent()

      await waitFor(() => {
        expect(screen.getByRole('heading', { name: 'Completed' })).toBeTruthy()
      })
    })

    it('displays skipped files warning', async () => {
      const completedJob = createMockJob({
        status: 'completed',
        filesCopied: 90,
        filesSkipped: 10,
      })
      mockInvoke.mockResolvedValue([completedJob])
      renderComponent()

      await waitFor(() => {
        expect(screen.getByText('10 files skipped')).toBeTruthy()
      })
    })

    it('shows remove button for completed jobs', async () => {
      const completedJob = createMockJob({ status: 'completed' })
      mockInvoke.mockResolvedValue([completedJob])
      renderComponent()

      await waitFor(() => {
        expect(screen.getByText('Remove')).toBeTruthy()
      })
    })

    it('does not show skipped files when zero', async () => {
      const completedJob = createMockJob({
        status: 'completed',
        filesCopied: 100,
        filesSkipped: 0,
      })
      mockInvoke.mockResolvedValue([completedJob])
      renderComponent()

      await waitFor(() => {
        expect(screen.queryByText(/files skipped/)).toBeNull()
      })
    })
  })

  describe('Job actions', () => {
    it('calls start_backup when Start Backup clicked', async () => {
      const user = userEvent.setup({ delay: null })
      const pendingJob = createMockJob({ status: 'pending' })
      mockInvoke.mockResolvedValue([pendingJob])
      renderComponent()

      await waitFor(() => {
        expect(screen.getByText('Start Backup')).toBeTruthy()
      })

      const startButton = screen.getByText('Start Backup')
      await user.click(startButton)

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('start_backup', { jobId: 'job-1' })
        expect(mockInvoke).toHaveBeenCalledWith('get_backup_queue')
      })
    })

    it('calls cancel_backup when Cancel clicked', async () => {
      const user = userEvent.setup({ delay: null })
      const pendingJob = createMockJob({ status: 'pending' })
      mockInvoke.mockResolvedValue([pendingJob])
      renderComponent()

      await waitFor(() => {
        expect(screen.getByText('Cancel')).toBeTruthy()
      })

      const cancelButton = screen.getByText('Cancel')
      await user.click(cancelButton)

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('cancel_backup', { jobId: 'job-1' })
        expect(mockInvoke).toHaveBeenCalledWith('get_backup_queue')
      })
    })

    it('calls remove_backup_job when Remove clicked', async () => {
      const user = userEvent.setup({ delay: null })
      const completedJob = createMockJob({ status: 'completed' })
      mockInvoke.mockResolvedValue([completedJob])
      renderComponent()

      await waitFor(() => {
        expect(screen.getByText('Remove')).toBeTruthy()
      })

      const removeButton = screen.getByText('Remove')
      await user.click(removeButton)

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('remove_backup_job', { jobId: 'job-1' })
        expect(mockInvoke).toHaveBeenCalledWith('get_backup_queue')
      })
    })

    it('handles start backup error gracefully', async () => {
      const user = userEvent.setup({ delay: null })
      const consoleError = vi.spyOn(console, 'error').mockImplementation(() => {})
      const pendingJob = createMockJob({ status: 'pending' })
      mockInvoke.mockResolvedValueOnce([pendingJob])
      mockInvoke.mockRejectedValueOnce(new Error('Failed to start'))

      renderComponent()

      await waitFor(() => {
        expect(screen.getByText('Start Backup')).toBeTruthy()
      })

      const startButton = screen.getByText('Start Backup')
      await user.click(startButton)

      await waitFor(() => {
        expect(consoleError).toHaveBeenCalledWith('Failed to start backup:', expect.any(Error))
      })

      consoleError.mockRestore()
    })

    it('handles cancel backup error gracefully', async () => {
      const user = userEvent.setup({ delay: null })
      const consoleError = vi.spyOn(console, 'error').mockImplementation(() => {})
      const pendingJob = createMockJob({ status: 'pending' })
      mockInvoke.mockResolvedValueOnce([pendingJob])
      mockInvoke.mockRejectedValueOnce(new Error('Failed to cancel'))

      renderComponent()

      await waitFor(() => {
        expect(screen.getByText('Cancel')).toBeTruthy()
      })

      const cancelButton = screen.getByText('Cancel')
      await user.click(cancelButton)

      await waitFor(() => {
        expect(consoleError).toHaveBeenCalledWith('Failed to cancel backup:', expect.any(Error))
      })

      consoleError.mockRestore()
    })

    it('handles remove job error gracefully', async () => {
      const user = userEvent.setup({ delay: null })
      const consoleError = vi.spyOn(console, 'error').mockImplementation(() => {})
      const completedJob = createMockJob({ status: 'completed' })
      mockInvoke.mockResolvedValueOnce([completedJob])
      mockInvoke.mockRejectedValueOnce(new Error('Failed to remove'))

      renderComponent()

      await waitFor(() => {
        expect(screen.getByText('Remove')).toBeTruthy()
      })

      const removeButton = screen.getByText('Remove')
      await user.click(removeButton)

      await waitFor(() => {
        expect(consoleError).toHaveBeenCalledWith('Failed to remove job:', expect.any(Error))
      })

      consoleError.mockRestore()
    })
  })

  describe('Event listeners', () => {
    it('sets up backup-progress event listener', async () => {
      renderComponent()

      await waitFor(() => {
        expect(mockListen).toHaveBeenCalledWith('backup-progress', expect.any(Function))
      })
    })

    it('sets up backup-job-updated event listener', async () => {
      renderComponent()

      await waitFor(() => {
        expect(mockListen).toHaveBeenCalledWith('backup-job-updated', expect.any(Function))
      })
    })

    it('updates job status on backup-job-updated event', async () => {
      const pendingJob = createMockJob({ id: 'job-1', status: 'pending' })
      mockInvoke.mockResolvedValue([pendingJob])

      let jobUpdateCallback: ((event: { payload: BackupJob }) => void) | null = null
      mockListen.mockImplementation((event: string, callback: (e: unknown) => void) => {
        if (event === 'backup-job-updated') {
          jobUpdateCallback = callback as (event: { payload: BackupJob }) => void
        }
        return Promise.resolve(() => {})
      })

      renderComponent()

      await waitFor(() => {
        expect(screen.getByRole('heading', { name: 'Pending' })).toBeTruthy()
      })

      const updatedJob = createMockJob({ id: 'job-1', status: 'inprogress' })
      if (jobUpdateCallback) {
        await act(async () => {
          jobUpdateCallback!({ payload: updatedJob })
        })
      }

      await waitFor(() => {
        expect(screen.getByRole('heading', { name: 'In Progress' })).toBeTruthy()
      })
    })

    it('cleans up event listeners on unmount', async () => {
      const mockUnlisten1 = vi.fn()
      const mockUnlisten2 = vi.fn()

      mockListen.mockImplementation((event: string) => {
        if (event === 'backup-progress') return Promise.resolve(mockUnlisten1)
        if (event === 'backup-job-updated') return Promise.resolve(mockUnlisten2)
        return Promise.resolve(() => {})
      })

      const { unmount } = renderComponent()

      await waitFor(() => {
        expect(mockListen).toHaveBeenCalled()
      })

      unmount()

      await waitFor(() => {
        expect(mockUnlisten1).toHaveBeenCalled()
        expect(mockUnlisten2).toHaveBeenCalled()
      })
    })
  })

  describe('Queue refresh', () => {
    it('loads queue on mount', async () => {
      renderComponent()

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('get_backup_queue')
      })
    })

    it('handles load queue error gracefully', async () => {
      const consoleError = vi.spyOn(console, 'error').mockImplementation(() => {})
      mockInvoke.mockRejectedValue(new Error('Failed to load'))

      renderComponent()

      await waitFor(() => {
        expect(consoleError).toHaveBeenCalledWith('Failed to load backup queue:', expect.any(Error))
      })

      consoleError.mockRestore()
    })

    it('refreshes queue on interval', async () => {
      vi.useFakeTimers()

      const { unmount } = renderComponent()

      // Wait for initial load
      expect(mockInvoke).toHaveBeenCalledWith('get_backup_queue')

      const initialCalls = mockInvoke.mock.calls.length

      // Advance timers
      act(() => {
        vi.advanceTimersByTime(30000)
      })

      // Should have called again
      expect(mockInvoke).toHaveBeenCalledTimes(initialCalls + 1)

      unmount()
      vi.useRealTimers()
    })

    it('clears interval on unmount', async () => {
      vi.useFakeTimers()

      const { unmount } = renderComponent()

      // Wait for initial load
      expect(mockInvoke).toHaveBeenCalledWith('get_backup_queue')

      const callsBeforeUnmount = mockInvoke.mock.calls.length

      unmount()

      act(() => {
        vi.advanceTimersByTime(30000)
      })

      // Should not have called again
      expect(mockInvoke).toHaveBeenCalledTimes(callsBeforeUnmount)

      vi.useRealTimers()
    })
  })

  describe('Multiple jobs with mixed statuses', () => {
    it('displays jobs in correct sections', async () => {
      const pendingJob = createMockJob({
        id: 'job-1',
        projectName: 'Pending Project',
        status: 'pending',
      })
      const activeJob = createMockJob({
        id: 'job-2',
        projectName: 'Active Project',
        status: 'inprogress',
      })
      const completedJob = createMockJob({
        id: 'job-3',
        projectName: 'Completed Project',
        status: 'completed',
      })

      mockInvoke.mockResolvedValue([pendingJob, activeJob, completedJob])
      renderComponent()

      await waitFor(() => {
        expect(screen.getByRole('heading', { name: 'In Progress' })).toBeTruthy()
        expect(screen.getByRole('heading', { name: 'Pending' })).toBeTruthy()
        expect(screen.getByRole('heading', { name: 'Completed' })).toBeTruthy()
        expect(screen.getByText('Pending Project')).toBeTruthy()
        expect(screen.getByText('Active Project')).toBeTruthy()
        expect(screen.getByText('Completed Project')).toBeTruthy()
      })
    })
  })
})
