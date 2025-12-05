import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import { userEvent } from '@testing-library/user-event'
import { Delivery } from './Delivery'
import { NotificationProvider } from '../contexts/NotificationContext'
import type { DeliveryDestination, DeliveryJob, Project, ProjectFile } from '../types'
import { ProjectStatus } from '../types'
import { invoke } from '@tauri-apps/api/core'

// Mock Tauri API
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockResolvedValue([]),
}))

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}))

vi.mock('@tauri-apps/plugin-dialog', () => ({
  open: vi.fn(),
}))

const mockInvoke = vi.mocked(invoke)

// Helper to create mock data
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

const createMockProjectFile = (overrides?: Partial<ProjectFile>): ProjectFile => ({
  name: 'test.jpg',
  path: '/path/to/test.jpg',
  relativePath: 'photos/test.jpg',
  size: 1_024_000,
  modified: '2024-01-15T10:00:00Z',
  type: 'image/jpeg',
  ...overrides,
})

const createMockDeliveryDestination = (
  overrides?: Partial<Extract<DeliveryDestination, { type: 'local' }>>
): DeliveryDestination => ({
  type: 'local',
  id: 'dest-1',
  name: 'Client Portal',
  path: '/Volumes/ClientDelivery',
  enabled: true,
  createdAt: '2024-01-01T00:00:00Z',
  ...overrides,
})

const createMockDeliveryJob = (overrides?: Partial<DeliveryJob>): DeliveryJob => ({
  id: 'job-1',
  projectId: '1',
  projectName: 'Test Project',
  selectedFiles: [],
  deliveryPath: '/Volumes/ClientDelivery',
  totalFiles: 10,
  filesCopied: 5,
  totalBytes: 10_240_000,
  bytesTransferred: 5_120_000,
  status: 'inprogress',
  createdAt: '2024-01-15T10:00:00Z',
  startedAt: '2024-01-15T10:01:00Z',
  completedAt: undefined,
  errorMessage: undefined,
  manifestPath: undefined,
  namingTemplate: undefined,
  ...overrides,
})

describe('delivery', () => {
  beforeEach(() => {
    mockInvoke.mockResolvedValue([])
    localStorage.clear()
    vi.clearAllMocks()
  })

  afterEach(() => {
    localStorage.clear()
  })

  describe('basic Rendering', () => {
    it('renders without crashing', async () => {
      render(
        <NotificationProvider>
          <Delivery />
        </NotificationProvider>
      )

      await waitFor(() => {
        expect(screen.getByRole('heading', { level: 1, name: 'Client Delivery' })).toBeTruthy()
      })
    })

    it('displays all delivery sections', async () => {
      render(
        <NotificationProvider>
          <Delivery />
        </NotificationProvider>
      )

      await waitFor(() => {
        expect(screen.getByText('1. Select Project')).toBeTruthy()
        expect(screen.getByText('Delivery Queue')).toBeTruthy()
      })
    })

    it('shows empty state when no deliveries', async () => {
      render(
        <NotificationProvider>
          <Delivery />
        </NotificationProvider>
      )

      await waitFor(() => {
        expect(screen.getByText('No deliveries yet')).toBeTruthy()
      })
    })
  })

  describe('project Selection', () => {
    it('shows project dropdown trigger', async () => {
      render(
        <NotificationProvider>
          <Delivery />
        </NotificationProvider>
      )

      await waitFor(() => {
        expect(screen.getByText('Choose a project...')).toBeTruthy()
      })
    })

    it('opens project dropdown when trigger clicked', async () => {
      const projects = [
        createMockProject({ id: '1', name: 'Project Alpha' }),
        createMockProject({ id: '2', name: 'Project Beta' }),
      ]

      mockInvoke.mockResolvedValue(projects)

      const user = userEvent.setup()

      render(
        <NotificationProvider>
          <Delivery />
        </NotificationProvider>
      )

      await waitFor(() => {
        expect(screen.getByText('Choose a project...')).toBeTruthy()
      })

      const trigger = screen.getByText('Choose a project...').closest('button')
      await user.click(trigger!)

      await waitFor(() => {
        expect(screen.getByText('Project Alpha')).toBeTruthy()
        expect(screen.getByText('Project Beta')).toBeTruthy()
      })
    })

    it('displays project metadata in dropdown', async () => {
      const project = createMockProject({
        clientName: 'John Doe',
        date: '2024-01-15',
        name: 'Wedding Shoot',
        shootType: 'Photography',
        status: ProjectStatus.Editing,
      })

      mockInvoke.mockResolvedValue([project])

      const user = userEvent.setup()

      render(
        <NotificationProvider>
          <Delivery />
        </NotificationProvider>
      )

      await waitFor(() => {
        expect(screen.getByText('Choose a project...')).toBeTruthy()
      })

      const trigger = screen.getByText('Choose a project...').closest('button')
      await user.click(trigger!)

      await waitFor(() => {
        expect(screen.getByText('Wedding Shoot')).toBeTruthy()
        const metadata = screen.getAllByText(/John Doe/)
        expect(metadata.length).toBeGreaterThan(0)
      })
    })

    it('selects project when clicked', async () => {
      const project = createMockProject({ name: 'Test Project' })

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'list_projects') {
          return Promise.resolve([project])
        }
        if (cmd === 'list_project_files') {
          return Promise.resolve([])
        }
        return Promise.resolve([])
      })

      const user = userEvent.setup()

      render(
        <NotificationProvider>
          <Delivery />
        </NotificationProvider>
      )

      await waitFor(() => {
        expect(screen.getByText('Choose a project...')).toBeTruthy()
      })

      const trigger = screen.getByText('Choose a project...').closest('button')
      await user.click(trigger!)

      await waitFor(() => {
        expect(screen.getByText('Test Project')).toBeTruthy()
      })

      const projectCard = screen.getByText('Test Project').closest('.project-select-card')
      await user.click(projectCard!)

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('list_project_files', { projectId: '1' })
      })
    })

    it('closes dropdown after selecting project', async () => {
      const project = createMockProject({ name: 'Test Project' })

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'list_projects') {
          return Promise.resolve([project])
        }
        if (cmd === 'list_project_files') {
          return Promise.resolve([])
        }
        return Promise.resolve([])
      })

      const user = userEvent.setup()

      render(
        <NotificationProvider>
          <Delivery />
        </NotificationProvider>
      )

      await waitFor(() => {
        expect(screen.getByText('Choose a project...')).toBeTruthy()
      })

      const trigger = screen.getByText('Choose a project...').closest('button')
      await user.click(trigger!)

      await waitFor(() => {
        expect(screen.getByText('Test Project')).toBeTruthy()
      })

      const projectCard = screen.getByText('Test Project').closest('.project-select-card')
      await user.click(projectCard!)

      await waitFor(() => {
        const dropdown = document.querySelector('.project-dropdown-list')
        expect(dropdown).toBeNull()
      })
    })

    it('shows empty state when no projects available', async () => {
      mockInvoke.mockResolvedValue([])

      const user = userEvent.setup()

      render(
        <NotificationProvider>
          <Delivery />
        </NotificationProvider>
      )

      await waitFor(() => {
        expect(screen.getByText('Choose a project...')).toBeTruthy()
      })

      const trigger = screen.getByText('Choose a project...').closest('button')
      await user.click(trigger!)

      await waitFor(() => {
        expect(screen.getByText('No projects available')).toBeTruthy()
      })
    })
  })

  describe('file Selection', () => {
    it('shows file selection section after project selected', async () => {
      const project = createMockProject()
      const files = [createMockProjectFile()]

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'list_projects') {
          return Promise.resolve([project])
        }
        if (cmd === 'list_project_files') {
          return Promise.resolve(files)
        }
        return Promise.resolve([])
      })

      const user = userEvent.setup()

      render(
        <NotificationProvider>
          <Delivery />
        </NotificationProvider>
      )

      await waitFor(() => {
        expect(screen.getByText('Choose a project...')).toBeTruthy()
      })

      const trigger = screen.getByText('Choose a project...').closest('button')
      await user.click(trigger!)

      await waitFor(() => {
        expect(screen.getByText('Test Project')).toBeTruthy()
      })

      const projectCard = screen.getByText('Test Project').closest('.project-select-card')
      await user.click(projectCard!)

      await waitFor(() => {
        expect(screen.getByText('2. Select Files')).toBeTruthy()
        expect(screen.getByText('Select All')).toBeTruthy()
        expect(screen.getByText('Deselect All')).toBeTruthy()
      })
    })

    it('displays project files', async () => {
      const project = createMockProject()
      const files = [
        createMockProjectFile({
          name: 'photo1.jpg',
          path: '/path/to/photo1.jpg',
          relativePath: 'photos/photo1.jpg',
        }),
        createMockProjectFile({
          name: 'photo2.jpg',
          path: '/path/to/photo2.jpg',
          relativePath: 'photos/photo2.jpg',
        }),
      ]

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'list_projects') {
          return Promise.resolve([project])
        }
        if (cmd === 'list_project_files') {
          return Promise.resolve(files)
        }
        return Promise.resolve([])
      })

      const user = userEvent.setup()

      render(
        <NotificationProvider>
          <Delivery />
        </NotificationProvider>
      )

      await waitFor(() => {
        expect(screen.getByText('Choose a project...')).toBeTruthy()
      })

      const trigger = screen.getByText('Choose a project...').closest('button')
      await user.click(trigger!)

      const projectCard = screen.getByText('Test Project').closest('.project-select-card')
      await user.click(projectCard!)

      await waitFor(() => {
        expect(screen.getByText('photo1.jpg')).toBeTruthy()
        expect(screen.getByText('photo2.jpg')).toBeTruthy()
        expect(screen.getByText(/photos\/photo1.jpg/)).toBeTruthy()
      })
    })

    it('shows empty state when no files', async () => {
      const project = createMockProject()

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'list_projects') {
          return Promise.resolve([project])
        }
        if (cmd === 'list_project_files') {
          return Promise.resolve([])
        }
        return Promise.resolve([])
      })

      const user = userEvent.setup()

      render(
        <NotificationProvider>
          <Delivery />
        </NotificationProvider>
      )

      await waitFor(() => {
        expect(screen.getByText('Choose a project...')).toBeTruthy()
      })

      const trigger = screen.getByText('Choose a project...').closest('button')
      await user.click(trigger!)

      const projectCard = screen.getByText('Test Project').closest('.project-select-card')
      await user.click(projectCard!)

      await waitFor(() => {
        expect(screen.getByText('No files found in project')).toBeTruthy()
      })
    })

    it('toggles file selection on click', async () => {
      const project = createMockProject()
      const files = [createMockProjectFile({ name: 'photo1.jpg' })]

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'list_projects') {
          return Promise.resolve([project])
        }
        if (cmd === 'list_project_files') {
          return Promise.resolve(files)
        }
        return Promise.resolve([])
      })

      const user = userEvent.setup()

      render(
        <NotificationProvider>
          <Delivery />
        </NotificationProvider>
      )

      await waitFor(() => {
        expect(screen.getByText('Choose a project...')).toBeTruthy()
      })

      const trigger = screen.getByText('Choose a project...').closest('button')
      await user.click(trigger!)

      const projectCard = screen.getByText('Test Project').closest('.project-select-card')
      await user.click(projectCard!)

      await waitFor(() => {
        expect(screen.getByText('photo1.jpg')).toBeTruthy()
        expect(screen.getByText('0 files selected')).toBeTruthy()
      })

      const fileItem = screen.getByText('photo1.jpg').closest('.file-item')
      await user.click(fileItem!)

      await waitFor(() => {
        expect(screen.getByText('1 file selected')).toBeTruthy()
      })
    })

    it('selects all files when Select All clicked', async () => {
      const project = createMockProject()
      const files = [
        createMockProjectFile({ name: 'photo1.jpg' }),
        createMockProjectFile({ name: 'photo2.jpg', path: '/path/to/photo2.jpg' }),
      ]

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'list_projects') {
          return Promise.resolve([project])
        }
        if (cmd === 'list_project_files') {
          return Promise.resolve(files)
        }
        return Promise.resolve([])
      })

      const user = userEvent.setup()

      render(
        <NotificationProvider>
          <Delivery />
        </NotificationProvider>
      )

      await waitFor(() => {
        expect(screen.getByText('Choose a project...')).toBeTruthy()
      })

      const trigger = screen.getByText('Choose a project...').closest('button')
      await user.click(trigger!)

      const projectCard = screen.getByText('Test Project').closest('.project-select-card')
      await user.click(projectCard!)

      await waitFor(() => {
        expect(screen.getByText('Select All')).toBeTruthy()
      })

      await user.click(screen.getByText('Select All'))

      await waitFor(() => {
        expect(screen.getByText('2 files selected')).toBeTruthy()
      })
    })

    it('deselects all files when Deselect All clicked', async () => {
      const project = createMockProject()
      const files = [createMockProjectFile({ name: 'photo1.jpg' })]

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'list_projects') {
          return Promise.resolve([project])
        }
        if (cmd === 'list_project_files') {
          return Promise.resolve(files)
        }
        return Promise.resolve([])
      })

      const user = userEvent.setup()

      render(
        <NotificationProvider>
          <Delivery />
        </NotificationProvider>
      )

      await waitFor(() => {
        expect(screen.getByText('Choose a project...')).toBeTruthy()
      })

      const trigger = screen.getByText('Choose a project...').closest('button')
      await user.click(trigger!)

      const projectCard = screen.getByText('Test Project').closest('.project-select-card')
      await user.click(projectCard!)

      await waitFor(() => {
        expect(screen.getByText('Select All')).toBeTruthy()
      })

      await user.click(screen.getByText('Select All'))

      await waitFor(() => {
        expect(screen.getByText('1 file selected')).toBeTruthy()
      })

      await user.click(screen.getByText('Deselect All'))

      await waitFor(() => {
        expect(screen.getByText('0 files selected')).toBeTruthy()
      })
    })
  })

  describe('naming Template', () => {
    it('shows naming template section after files selected', async () => {
      const project = createMockProject()
      const files = [createMockProjectFile()]

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'list_projects') {
          return Promise.resolve([project])
        }
        if (cmd === 'list_project_files') {
          return Promise.resolve(files)
        }
        return Promise.resolve([])
      })

      const user = userEvent.setup()

      render(
        <NotificationProvider>
          <Delivery />
        </NotificationProvider>
      )

      await waitFor(() => {
        expect(screen.getByText('Choose a project...')).toBeTruthy()
      })

      const trigger = screen.getByText('Choose a project...').closest('button')
      await user.click(trigger!)

      const projectCard = screen.getByText('Test Project').closest('.project-select-card')
      await user.click(projectCard!)

      await waitFor(() => {
        expect(screen.getByText('Select All')).toBeTruthy()
      })

      await user.click(screen.getByText('Select All'))

      await waitFor(() => {
        expect(screen.getByText('3. Naming Template (Optional)')).toBeTruthy()
      })
    })

    it('allows entering naming template', async () => {
      const project = createMockProject()
      const files = [createMockProjectFile()]

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'list_projects') {
          return Promise.resolve([project])
        }
        if (cmd === 'list_project_files') {
          return Promise.resolve(files)
        }
        return Promise.resolve([])
      })

      const user = userEvent.setup()

      render(
        <NotificationProvider>
          <Delivery />
        </NotificationProvider>
      )

      await waitFor(() => {
        expect(screen.getByText('Choose a project...')).toBeTruthy()
      })

      const trigger = screen.getByText('Choose a project...').closest('button')
      await user.click(trigger!)

      const projectCard = screen.getByText('Test Project').closest('.project-select-card')
      await user.click(projectCard!)

      await waitFor(() => {
        expect(screen.getByText('Select All')).toBeTruthy()
      })

      await user.click(screen.getByText('Select All'))

      await waitFor(() => {
        expect(screen.getByText('3. Naming Template (Optional)')).toBeTruthy()
      })

      const input = screen.getByPlaceholderText(/e.g.,/)
      await user.type(input, 'photo_001.jpg')

      expect(input).toBeInstanceOf(HTMLInputElement)
      expect(input).toHaveProperty('value', 'photo_001.jpg')
    })
  })

  describe('destination Selection', () => {
    it('shows destination section after files selected', async () => {
      const project = createMockProject()
      const files = [createMockProjectFile()]
      const destination = createMockDeliveryDestination()

      localStorage.setItem('delivery_destinations', JSON.stringify([destination]))

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'list_projects') {
          return Promise.resolve([project])
        }
        if (cmd === 'list_project_files') {
          return Promise.resolve(files)
        }
        return Promise.resolve([])
      })

      const user = userEvent.setup()

      render(
        <NotificationProvider>
          <Delivery />
        </NotificationProvider>
      )

      await waitFor(() => {
        expect(screen.getByText('Choose a project...')).toBeTruthy()
      })

      const trigger = screen.getByText('Choose a project...').closest('button')
      await user.click(trigger!)

      const projectCard = screen.getByText('Test Project').closest('.project-select-card')
      await user.click(projectCard!)

      await waitFor(() => {
        expect(screen.getByText('Select All')).toBeTruthy()
      })

      await user.click(screen.getByText('Select All'))

      await waitFor(() => {
        expect(screen.getByText('4. Choose Destination')).toBeTruthy()
        expect(screen.getByText('Client Portal')).toBeTruthy()
      })
    })

    it('shows empty state when no destinations configured', async () => {
      const project = createMockProject()
      const files = [createMockProjectFile()]

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'list_projects') {
          return Promise.resolve([project])
        }
        if (cmd === 'list_project_files') {
          return Promise.resolve(files)
        }
        return Promise.resolve([])
      })

      const user = userEvent.setup()

      render(
        <NotificationProvider>
          <Delivery />
        </NotificationProvider>
      )

      await waitFor(() => {
        expect(screen.getByText('Choose a project...')).toBeTruthy()
      })

      const trigger = screen.getByText('Choose a project...').closest('button')
      await user.click(trigger!)

      const projectCard = screen.getByText('Test Project').closest('.project-select-card')
      await user.click(projectCard!)

      await waitFor(() => {
        expect(screen.getByText('Select All')).toBeTruthy()
      })

      await user.click(screen.getByText('Select All'))

      await waitFor(() => {
        expect(screen.getByText('No delivery destinations configured')).toBeTruthy()
      })
    })

    it('filters out disabled destinations', async () => {
      const project = createMockProject()
      const files = [createMockProjectFile()]
      const destinations = [
        createMockDeliveryDestination({ enabled: true, name: 'Enabled Dest' }),
        createMockDeliveryDestination({
          enabled: false,
          id: 'dest-2',
          name: 'Disabled Dest',
        }),
      ]

      localStorage.setItem('delivery_destinations', JSON.stringify(destinations))

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'list_projects') {
          return Promise.resolve([project])
        }
        if (cmd === 'list_project_files') {
          return Promise.resolve(files)
        }
        return Promise.resolve([])
      })

      const user = userEvent.setup()

      render(
        <NotificationProvider>
          <Delivery />
        </NotificationProvider>
      )

      await waitFor(() => {
        expect(screen.getByText('Choose a project...')).toBeTruthy()
      })

      const trigger = screen.getByText('Choose a project...').closest('button')
      await user.click(trigger!)

      const projectCard = screen.getByText('Test Project').closest('.project-select-card')
      await user.click(projectCard!)

      await waitFor(() => {
        expect(screen.getByText('Select All')).toBeTruthy()
      })

      await user.click(screen.getByText('Select All'))

      await waitFor(() => {
        expect(screen.getByText('Enabled Dest')).toBeTruthy()
        expect(screen.queryByText('Disabled Dest')).toBeNull()
      })
    })

    it('creates delivery job when destination clicked', async () => {
      const project = createMockProject()
      const files = [createMockProjectFile()]
      const destination = createMockDeliveryDestination()
      const job = createMockDeliveryJob()

      localStorage.setItem('delivery_destinations', JSON.stringify([destination]))

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'list_projects') {
          return Promise.resolve([project])
        }
        if (cmd === 'list_project_files') {
          return Promise.resolve(files)
        }
        if (cmd === 'create_delivery') {
          return Promise.resolve(job)
        }
        if (cmd === 'start_delivery') {
          return Promise.resolve()
        }
        return Promise.resolve([])
      })

      const user = userEvent.setup()

      render(
        <NotificationProvider>
          <Delivery />
        </NotificationProvider>
      )

      await waitFor(() => {
        expect(screen.getByText('Choose a project...')).toBeTruthy()
      })

      const trigger = screen.getByText('Choose a project...').closest('button')
      await user.click(trigger!)

      const projectCard = screen.getByText('Test Project').closest('.project-select-card')
      await user.click(projectCard!)

      await waitFor(() => {
        expect(screen.getByText('Select All')).toBeTruthy()
      })

      await user.click(screen.getByText('Select All'))

      await waitFor(() => {
        expect(screen.getByText('Client Portal')).toBeTruthy()
      })

      await user.click(screen.getByText('Client Portal'))

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('create_delivery', expect.any(Object))
        expect(mockInvoke).toHaveBeenCalledWith('start_delivery', { jobId: 'job-1' })
      })
    })
  })

  describe('delivery Queue', () => {
    it('displays delivery jobs', async () => {
      const job = createMockDeliveryJob({ projectName: 'Wedding Shoot' })

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_delivery_queue') {
          return Promise.resolve([job])
        }
        return Promise.resolve([])
      })

      render(
        <NotificationProvider>
          <Delivery />
        </NotificationProvider>
      )

      await waitFor(() => {
        expect(screen.getByText('Wedding Shoot')).toBeTruthy()
      })
    })

    it('displays job status', async () => {
      const job = createMockDeliveryJob({ status: 'inprogress' })

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_delivery_queue') {
          return Promise.resolve([job])
        }
        return Promise.resolve([])
      })

      render(
        <NotificationProvider>
          <Delivery />
        </NotificationProvider>
      )

      await waitFor(() => {
        expect(screen.getByText('inprogress')).toBeTruthy()
      })
    })

    it('displays job progress', async () => {
      const job = createMockDeliveryJob({
        bytesTransferred: 5_120_000,
        filesCopied: 5,
        totalBytes: 10_240_000,
        totalFiles: 10,
      })

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_delivery_queue') {
          return Promise.resolve([job])
        }
        return Promise.resolve([])
      })

      render(
        <NotificationProvider>
          <Delivery />
        </NotificationProvider>
      )

      await waitFor(() => {
        expect(screen.getByText('5 / 10')).toBeTruthy()
      })
    })

    it('shows progress bar for in-progress jobs', async () => {
      const job = createMockDeliveryJob({
        bytesTransferred: 5_120_000,
        status: 'inprogress',
        totalBytes: 10_240_000,
      })

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_delivery_queue') {
          return Promise.resolve([job])
        }
        return Promise.resolve([])
      })

      render(
        <NotificationProvider>
          <Delivery />
        </NotificationProvider>
      )

      await waitFor(() => {
        const progressBar = document.querySelector('.progress-bar')
        expect(progressBar).toBeTruthy()
      })
    })

    it('shows remove button for completed jobs', async () => {
      const job = createMockDeliveryJob({ status: 'completed' })

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_delivery_queue') {
          return Promise.resolve([job])
        }
        return Promise.resolve([])
      })

      render(
        <NotificationProvider>
          <Delivery />
        </NotificationProvider>
      )

      await waitFor(() => {
        expect(screen.getByText('Remove')).toBeTruthy()
      })
    })

    it('shows remove button for failed jobs', async () => {
      const job = createMockDeliveryJob({ status: 'failed' })

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_delivery_queue') {
          return Promise.resolve([job])
        }
        return Promise.resolve([])
      })

      render(
        <NotificationProvider>
          <Delivery />
        </NotificationProvider>
      )

      await waitFor(() => {
        expect(screen.getByText('Remove')).toBeTruthy()
      })
    })

    it('removes job when remove button clicked', async () => {
      const job = createMockDeliveryJob({ id: 'job-1', status: 'completed' })

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_delivery_queue') {
          return Promise.resolve([job])
        }
        if (cmd === 'remove_delivery_job') {
          return Promise.resolve()
        }
        return Promise.resolve([])
      })

      const user = userEvent.setup()

      render(
        <NotificationProvider>
          <Delivery />
        </NotificationProvider>
      )

      await waitFor(() => {
        expect(screen.getByText('Remove')).toBeTruthy()
      })

      await user.click(screen.getByText('Remove'))

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('remove_delivery_job', { jobId: 'job-1' })
      })
    })

    it('displays error message for failed jobs', async () => {
      const job = createMockDeliveryJob({
        errorMessage: 'Disk full',
        status: 'failed',
      })

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_delivery_queue') {
          return Promise.resolve([job])
        }
        return Promise.resolve([])
      })

      render(
        <NotificationProvider>
          <Delivery />
        </NotificationProvider>
      )

      await waitFor(() => {
        expect(screen.getByText('Disk full')).toBeTruthy()
      })
    })

    it('displays manifest path when available', async () => {
      const job = createMockDeliveryJob({
        manifestPath: '/path/to/manifest.txt',
      })

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_delivery_queue') {
          return Promise.resolve([job])
        }
        return Promise.resolve([])
      })

      render(
        <NotificationProvider>
          <Delivery />
        </NotificationProvider>
      )

      await waitFor(() => {
        expect(screen.getByText(/Manifest:/)).toBeTruthy()
        expect(screen.getByText(/manifest.txt/)).toBeTruthy()
      })
    })
  })

  describe('status Helpers', () => {
    it('applies correct CSS class for pending status', async () => {
      const job = createMockDeliveryJob({ status: 'pending' })

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_delivery_queue') {
          return Promise.resolve([job])
        }
        return Promise.resolve([])
      })

      render(
        <NotificationProvider>
          <Delivery />
        </NotificationProvider>
      )

      await waitFor(() => {
        const statusElement = screen.getByText('pending')
        expect(statusElement.className).toContain('status-pending')
      })
    })

    it('applies correct CSS class for completed status', async () => {
      const job = createMockDeliveryJob({ status: 'completed' })

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_delivery_queue') {
          return Promise.resolve([job])
        }
        return Promise.resolve([])
      })

      render(
        <NotificationProvider>
          <Delivery />
        </NotificationProvider>
      )

      await waitFor(() => {
        const statusElement = screen.getByText('completed')
        expect(statusElement.className).toContain('status-completed')
      })
    })

    it('applies correct CSS class for failed status', async () => {
      const job = createMockDeliveryJob({ status: 'failed' })

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_delivery_queue') {
          return Promise.resolve([job])
        }
        return Promise.resolve([])
      })

      render(
        <NotificationProvider>
          <Delivery />
        </NotificationProvider>
      )

      await waitFor(() => {
        const statusElement = screen.getByText('failed')
        expect(statusElement.className).toContain('status-failed')
      })
    })
  })
})
