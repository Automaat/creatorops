import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import { userEvent } from '@testing-library/user-event'
import { Projects } from './Projects'
import { NotificationProvider } from '../contexts/NotificationContext'
import type { BackupDestination, ImportHistory, Project } from '../types'
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

vi.mock('../hooks/useSDCardScanner', () => ({
  useSDCardScanner: () => ({
    isScanning: false,
    sdCards: [],
  }),
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

const createMockBackupDestination = (
  overrides?: Partial<BackupDestination>
): BackupDestination => ({
  id: 'dest-1',
  name: 'External Drive',
  path: '/Volumes/Backup',
  enabled: true,
  createdAt: '2024-01-01T00:00:00Z',
  ...overrides,
})

describe('projects', () => {
  beforeEach(() => {
    mockInvoke.mockResolvedValue([])
    localStorage.clear()
    vi.clearAllMocks()
  })

  afterEach(() => {
    localStorage.clear()
  })

  describe('list View', () => {
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

    it('renders multiple projects in list', async () => {
      const projects = [
        createMockProject({ id: '1', name: 'Project Alpha' }),
        createMockProject({ id: '2', name: 'Project Beta' }),
        createMockProject({ id: '3', name: 'Project Gamma' }),
      ]

      mockInvoke.mockResolvedValue(projects)

      render(
        <NotificationProvider>
          <Projects />
        </NotificationProvider>
      )

      await waitFor(() => {
        expect(screen.getByText('Project Alpha')).toBeTruthy()
        expect(screen.getByText('Project Beta')).toBeTruthy()
        expect(screen.getByText('Project Gamma')).toBeTruthy()
      })
    })

    it('displays project metadata in cards', async () => {
      const project = createMockProject({
        clientName: 'John Doe',
        date: '2024-01-15',
        name: 'Wedding Shoot',
        shootType: 'Wedding',
      })

      mockInvoke.mockResolvedValue([project])

      render(
        <NotificationProvider>
          <Projects />
        </NotificationProvider>
      )

      await waitFor(() => {
        expect(screen.getByText('Wedding Shoot')).toBeTruthy()
        expect(screen.getByText('John Doe')).toBeTruthy()
        expect(screen.getByText('Wedding')).toBeTruthy()
        expect(screen.getByText('2024-01-15')).toBeTruthy()
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

    it('shows Create Project button', async () => {
      render(
        <NotificationProvider>
          <Projects />
        </NotificationProvider>
      )

      await waitFor(() => {
        expect(screen.getByText('Create Project')).toBeTruthy()
      })
    })

    it('opens create project dialog when button clicked', async () => {
      const user = userEvent.setup()

      render(
        <NotificationProvider>
          <Projects />
        </NotificationProvider>
      )

      await waitFor(() => {
        expect(screen.getByText('Create Project')).toBeTruthy()
      })

      await user.click(screen.getByText('Create Project'))

      await waitFor(() => {
        expect(screen.getByText('Create New Project')).toBeTruthy()
      })
    })
  })

  describe('project Selection and Detail View', () => {
    it('shows project detail when card is clicked', async () => {
      const project = createMockProject({ name: 'Test Project' })
      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'list_projects') {
          return Promise.resolve([project])
        }
        if (cmd === 'get_project_import_history') {
          return Promise.resolve([])
        }
        if (cmd === 'get_home_directory') {
          return Promise.resolve('/Users/test')
        }
        return Promise.resolve([])
      })

      const user = userEvent.setup()

      render(
        <NotificationProvider>
          <Projects />
        </NotificationProvider>
      )

      await waitFor(() => {
        expect(screen.getByText('Test Project')).toBeTruthy()
      })

      const projectCard = screen.getByText('Test Project').closest('.project-card')
      expect(projectCard).toBeTruthy()

      await user.click(projectCard!)

      await waitFor(() => {
        expect(screen.getByText('← Back')).toBeTruthy()
        expect(screen.getByText('Actions')).toBeTruthy()
      })
    })

    it('returns to list view when back button clicked', async () => {
      const project = createMockProject({ name: 'Test Project' })
      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'list_projects') {
          return Promise.resolve([project])
        }
        if (cmd === 'get_project_import_history') {
          return Promise.resolve([])
        }
        if (cmd === 'get_home_directory') {
          return Promise.resolve('/Users/test')
        }
        return Promise.resolve([])
      })

      const user = userEvent.setup()

      render(
        <NotificationProvider>
          <Projects />
        </NotificationProvider>
      )

      await waitFor(() => {
        expect(screen.getByText('Test Project')).toBeTruthy()
      })

      const projectCard = screen.getByText('Test Project').closest('.project-card')
      await user.click(projectCard!)

      await waitFor(() => {
        expect(screen.getByText('← Back')).toBeTruthy()
      })

      await user.click(screen.getByText('← Back'))

      await waitFor(() => {
        expect(screen.getByText('Projects')).toBeTruthy()
        expect(screen.queryByText('Actions')).toBeNull()
      })
    })

    it('loads project from initialSelectedProjectId prop', async () => {
      const project = createMockProject({ id: 'proj-123', name: 'Initial Project' })
      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_project') {
          return Promise.resolve(project)
        }
        if (cmd === 'get_project_import_history') {
          return Promise.resolve([])
        }
        if (cmd === 'get_home_directory') {
          return Promise.resolve('/Users/test')
        }
        return Promise.resolve([])
      })

      render(
        <NotificationProvider>
          <Projects initialSelectedProjectId="proj-123" />
        </NotificationProvider>
      )

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('get_project', { projectId: 'proj-123' })
        expect(screen.getByText('Initial Project')).toBeTruthy()
        expect(screen.getByText('← Back')).toBeTruthy()
      })
    })

    it('calls onBackFromProject when back button clicked from external selection', async () => {
      const project = createMockProject({ id: 'proj-123', name: 'External Project' })
      const onBackFromProject = vi.fn()

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_project') {
          return Promise.resolve(project)
        }
        if (cmd === 'get_project_import_history') {
          return Promise.resolve([])
        }
        if (cmd === 'get_home_directory') {
          return Promise.resolve('/Users/test')
        }
        return Promise.resolve([])
      })

      const user = userEvent.setup()

      render(
        <NotificationProvider>
          <Projects initialSelectedProjectId="proj-123" onBackFromProject={onBackFromProject} />
        </NotificationProvider>
      )

      await waitFor(() => {
        expect(screen.getByText('← Back')).toBeTruthy()
      })

      await user.click(screen.getByText('← Back'))

      expect(onBackFromProject).toHaveBeenCalledTimes(1)
    })
  })

  describe('project Detail View', () => {
    it('displays project metadata in detail view', async () => {
      const project = createMockProject({
        clientName: 'John Doe',
        date: '2024-01-15',
        name: 'Wedding Shoot',
        shootType: 'Wedding',
        status: ProjectStatus.Editing,
      })

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_project') {
          return Promise.resolve(project)
        }
        if (cmd === 'get_project_import_history') {
          return Promise.resolve([])
        }
        if (cmd === 'get_home_directory') {
          return Promise.resolve('/Users/test')
        }
        return Promise.resolve([])
      })

      render(
        <NotificationProvider>
          <Projects initialSelectedProjectId="1" />
        </NotificationProvider>
      )

      await waitFor(() => {
        expect(screen.getByText('Wedding Shoot')).toBeTruthy()
        expect(screen.getByText('John Doe')).toBeTruthy()
        expect(screen.getByText('Wedding')).toBeTruthy()
        expect(screen.getByText('2024-01-15')).toBeTruthy()
        expect(screen.getByText('Editing')).toBeTruthy()
      })
    })

    it('displays project folder path', async () => {
      const project = createMockProject({ folderPath: '/Users/test/projects/test-project' })

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_project') {
          return Promise.resolve(project)
        }
        if (cmd === 'get_project_import_history') {
          return Promise.resolve([])
        }
        if (cmd === 'get_home_directory') {
          return Promise.resolve('/Users/test')
        }
        return Promise.resolve([])
      })

      render(
        <NotificationProvider>
          <Projects initialSelectedProjectId="1" />
        </NotificationProvider>
      )

      await waitFor(() => {
        expect(screen.getByText('~/projects/test-project')).toBeTruthy()
      })
    })

    it('opens reveal in finder when folder path clicked', async () => {
      const project = createMockProject({ folderPath: '/path/to/project' })

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_project') {
          return Promise.resolve(project)
        }
        if (cmd === 'get_project_import_history') {
          return Promise.resolve([])
        }
        if (cmd === 'get_home_directory') {
          return Promise.resolve('/Users/test')
        }
        return Promise.resolve([])
      })

      const user = userEvent.setup()

      render(
        <NotificationProvider>
          <Projects initialSelectedProjectId="1" />
        </NotificationProvider>
      )

      await waitFor(() => {
        expect(screen.getByTitle('Click to show in Finder')).toBeTruthy()
      })

      await user.click(screen.getByTitle('Click to show in Finder'))

      expect(mockInvoke).toHaveBeenCalledWith('reveal_in_finder', { path: '/path/to/project' })
    })

    it('shows Import button in detail view', async () => {
      const project = createMockProject()

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_project') {
          return Promise.resolve(project)
        }
        if (cmd === 'get_project_import_history') {
          return Promise.resolve([])
        }
        if (cmd === 'get_home_directory') {
          return Promise.resolve('/Users/test')
        }
        return Promise.resolve([])
      })

      render(
        <NotificationProvider>
          <Projects initialSelectedProjectId="1" />
        </NotificationProvider>
      )

      await waitFor(() => {
        expect(screen.getByText('Import')).toBeTruthy()
      })
    })
  })

  describe('archive Functionality', () => {
    it('shows archive button when project not archived', async () => {
      const project = createMockProject({ status: ProjectStatus.Editing })

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_project') {
          return Promise.resolve(project)
        }
        if (cmd === 'get_project_import_history') {
          return Promise.resolve([])
        }
        if (cmd === 'get_home_directory') {
          return Promise.resolve('/Users/test')
        }
        return Promise.resolve([])
      })

      render(
        <NotificationProvider>
          <Projects initialSelectedProjectId="1" />
        </NotificationProvider>
      )

      await waitFor(() => {
        expect(screen.getByText('Archive')).toBeTruthy()
      })
    })

    it('disables archive button when project already archived', async () => {
      const project = createMockProject({ status: ProjectStatus.Archived })

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_project') {
          return Promise.resolve(project)
        }
        if (cmd === 'get_project_import_history') {
          return Promise.resolve([])
        }
        if (cmd === 'get_home_directory') {
          return Promise.resolve('/Users/test')
        }
        return Promise.resolve([])
      })

      render(
        <NotificationProvider>
          <Projects initialSelectedProjectId="1" />
        </NotificationProvider>
      )

      await waitFor(() => {
        const button = screen.getByText('Already Archived')
        expect(button).toBeTruthy()
        if (button instanceof HTMLButtonElement) {
          expect(button.disabled).toBe(true)
        }
      })
    })

    it('opens archive dialog when archive button clicked', async () => {
      const project = createMockProject({ status: ProjectStatus.Editing })
      localStorage.setItem('archive_location', '/path/to/archives')

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_project') {
          return Promise.resolve(project)
        }
        if (cmd === 'get_project_import_history') {
          return Promise.resolve([])
        }
        if (cmd === 'get_home_directory') {
          return Promise.resolve('/Users/test')
        }
        return Promise.resolve([])
      })

      const user = userEvent.setup()

      render(
        <NotificationProvider>
          <Projects initialSelectedProjectId="1" />
        </NotificationProvider>
      )

      await waitFor(() => {
        expect(screen.getByText('Archive')).toBeTruthy()
      })

      await user.click(screen.getByText('Archive'))

      await waitFor(() => {
        expect(screen.getAllByText('Archive Project').length).toBeGreaterThan(0)
        expect(screen.getByText(/Archive Location:/)).toBeTruthy()
        expect(screen.getByText('Confirm Archive')).toBeTruthy()
      })
    })

    it('closes archive dialog when cancel clicked', async () => {
      const project = createMockProject({ status: ProjectStatus.Editing })
      localStorage.setItem('archive_location', '/path/to/archives')

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_project') {
          return Promise.resolve(project)
        }
        if (cmd === 'get_project_import_history') {
          return Promise.resolve([])
        }
        if (cmd === 'get_home_directory') {
          return Promise.resolve('/Users/test')
        }
        return Promise.resolve([])
      })

      const user = userEvent.setup()

      render(
        <NotificationProvider>
          <Projects initialSelectedProjectId="1" />
        </NotificationProvider>
      )

      await waitFor(() => {
        expect(screen.getByText('Archive')).toBeTruthy()
      })

      await user.click(screen.getByText('Archive'))

      await waitFor(() => {
        expect(screen.getByText('Confirm Archive')).toBeTruthy()
      })

      await user.click(screen.getByText('Cancel'))

      await waitFor(() => {
        expect(screen.queryByText('Confirm Archive')).toBeNull()
      })
    })
  })

  describe('delete Functionality', () => {
    it('shows delete button in detail view', async () => {
      const project = createMockProject()

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_project') {
          return Promise.resolve(project)
        }
        if (cmd === 'get_project_import_history') {
          return Promise.resolve([])
        }
        if (cmd === 'get_home_directory') {
          return Promise.resolve('/Users/test')
        }
        return Promise.resolve([])
      })

      render(
        <NotificationProvider>
          <Projects initialSelectedProjectId="1" />
        </NotificationProvider>
      )

      await waitFor(() => {
        const buttons = screen.getAllByText('Delete Project')
        expect(buttons.length).toBeGreaterThan(0)
      })
    })

    it('opens delete confirmation dialog when delete clicked', async () => {
      const project = createMockProject({ name: 'Test Project' })

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_project') {
          return Promise.resolve(project)
        }
        if (cmd === 'get_project_import_history') {
          return Promise.resolve([])
        }
        if (cmd === 'get_home_directory') {
          return Promise.resolve('/Users/test')
        }
        return Promise.resolve([])
      })

      const user = userEvent.setup()

      render(
        <NotificationProvider>
          <Projects initialSelectedProjectId="1" />
        </NotificationProvider>
      )

      await waitFor(() => {
        const buttons = screen.getAllByText('Delete Project')
        expect(buttons.length).toBeGreaterThan(0)
      })

      const deleteButtons = screen.getAllByText('Delete Project')
      const deleteButton = deleteButtons.find((btn) => btn.classList.contains('btn-danger'))
      await user.click(deleteButton!)

      await waitFor(() => {
        expect(screen.getByText(/Are you sure you want to delete/)).toBeTruthy()
        expect(screen.getByText(/This action cannot be undone/)).toBeTruthy()
      })
    })

    it('closes delete dialog when cancel clicked', async () => {
      const project = createMockProject()

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_project') {
          return Promise.resolve(project)
        }
        if (cmd === 'get_project_import_history') {
          return Promise.resolve([])
        }
        if (cmd === 'get_home_directory') {
          return Promise.resolve('/Users/test')
        }
        return Promise.resolve([])
      })

      const user = userEvent.setup()

      render(
        <NotificationProvider>
          <Projects initialSelectedProjectId="1" />
        </NotificationProvider>
      )

      await waitFor(() => {
        const buttons = screen.getAllByText('Delete Project')
        expect(buttons.length).toBeGreaterThan(0)
      })

      const deleteButtons = screen.getAllByText('Delete Project')
      const deleteButton = deleteButtons.find((btn) => btn.classList.contains('btn-danger'))
      await user.click(deleteButton!)

      await waitFor(() => {
        expect(screen.getByText(/Are you sure/)).toBeTruthy()
      })

      const cancelButtons = screen.getAllByText('Cancel')
      const lastCancel = cancelButtons.at(-1)
      if (lastCancel) {
        await user.click(lastCancel)
      }

      await waitFor(() => {
        expect(screen.queryByText(/Are you sure/)).toBeNull()
      })
    })

    it('deletes project when confirmed', async () => {
      const project = createMockProject({ id: 'proj-123', name: 'Test Project' })

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_project') {
          return Promise.resolve(project)
        }
        if (cmd === 'get_project_import_history') {
          return Promise.resolve([])
        }
        if (cmd === 'get_home_directory') {
          return Promise.resolve('/Users/test')
        }
        if (cmd === 'delete_project') {
          return Promise.resolve()
        }
        if (cmd === 'list_projects') {
          return Promise.resolve([])
        }
        return Promise.resolve([])
      })

      const user = userEvent.setup()

      render(
        <NotificationProvider>
          <Projects initialSelectedProjectId="proj-123" />
        </NotificationProvider>
      )

      await waitFor(() => {
        const buttons = screen.getAllByText('Delete Project')
        expect(buttons.length).toBeGreaterThan(0)
      })

      const initialDeleteButtons = screen.getAllByText('Delete Project')
      const deleteButton = initialDeleteButtons.find((btn) => btn.classList.contains('btn-danger'))
      await user.click(deleteButton!)

      await waitFor(() => {
        expect(screen.getByText(/Are you sure/)).toBeTruthy()
      })

      const deleteButtons = screen.getAllByText('Delete Project')
      const lastDeleteButton = deleteButtons.at(-1)
      if (lastDeleteButton) {
        await user.click(lastDeleteButton)
      }

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('delete_project', { projectId: 'proj-123' })
      })
    })
  })

  describe('open in App Functionality', () => {
    it('shows editing app buttons', async () => {
      const project = createMockProject()

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_project') {
          return Promise.resolve(project)
        }
        if (cmd === 'get_project_import_history') {
          return Promise.resolve([])
        }
        if (cmd === 'get_home_directory') {
          return Promise.resolve('/Users/test')
        }
        return Promise.resolve([])
      })

      render(
        <NotificationProvider>
          <Projects initialSelectedProjectId="1" />
        </NotificationProvider>
      )

      await waitFor(() => {
        expect(screen.getByText('Lightroom Classic')).toBeTruthy()
        expect(screen.getByText('AfterShoot')).toBeTruthy()
        expect(screen.getByText('DaVinci Resolve')).toBeTruthy()
        expect(screen.getByText('Final Cut Pro')).toBeTruthy()
      })
    })

    it('calls open_in_lightroom when Lightroom Classic clicked', async () => {
      const project = createMockProject({ folderPath: '/path/to/project' })

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_project') {
          return Promise.resolve(project)
        }
        if (cmd === 'get_project_import_history') {
          return Promise.resolve([])
        }
        if (cmd === 'get_home_directory') {
          return Promise.resolve('/Users/test')
        }
        if (cmd === 'open_in_lightroom') {
          return Promise.resolve()
        }
        return Promise.resolve([])
      })

      const user = userEvent.setup()

      render(
        <NotificationProvider>
          <Projects initialSelectedProjectId="1" />
        </NotificationProvider>
      )

      await waitFor(() => {
        expect(screen.getByText('Lightroom Classic')).toBeTruthy()
      })

      await user.click(screen.getByText('Lightroom Classic'))

      expect(mockInvoke).toHaveBeenCalledWith('open_in_lightroom', { path: '/path/to/project' })
    })

    it('updates project status to Editing when opening app from New status', async () => {
      const project = createMockProject({ status: ProjectStatus.New })
      const updatedProject = { ...project, status: ProjectStatus.Editing }

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_project') {
          return Promise.resolve(project)
        }
        if (cmd === 'get_project_import_history') {
          return Promise.resolve([])
        }
        if (cmd === 'get_home_directory') {
          return Promise.resolve('/Users/test')
        }
        if (cmd === 'open_in_lightroom') {
          return Promise.resolve()
        }
        if (cmd === 'update_project_status') {
          return Promise.resolve(updatedProject)
        }
        return Promise.resolve([])
      })

      const user = userEvent.setup()

      render(
        <NotificationProvider>
          <Projects initialSelectedProjectId="1" />
        </NotificationProvider>
      )

      await waitFor(() => {
        expect(screen.getByText('Lightroom Classic')).toBeTruthy()
      })

      await user.click(screen.getByText('Lightroom Classic'))

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('update_project_status', {
          newStatus: ProjectStatus.Editing,
          projectId: '1',
        })
      })
    })
  })

  describe('backup Functionality', () => {
    it('shows backup destinations when configured', async () => {
      const project = createMockProject()
      const destinations = [
        createMockBackupDestination({ name: 'External Drive', path: '/Volumes/Backup' }),
        createMockBackupDestination({ id: 'dest-2', name: 'NAS Drive', path: '/Volumes/NAS' }),
      ]

      localStorage.setItem('backup_destinations', JSON.stringify(destinations))

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_project') {
          return Promise.resolve(project)
        }
        if (cmd === 'get_project_import_history') {
          return Promise.resolve([])
        }
        if (cmd === 'get_home_directory') {
          return Promise.resolve('/Users/test')
        }
        return Promise.resolve([])
      })

      render(
        <NotificationProvider>
          <Projects initialSelectedProjectId="1" />
        </NotificationProvider>
      )

      await waitFor(() => {
        expect(screen.getByText('External Drive')).toBeTruthy()
        expect(screen.getByText('NAS Drive')).toBeTruthy()
      })
    })

    it('shows empty state when no backup destinations', async () => {
      const project = createMockProject()

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_project') {
          return Promise.resolve(project)
        }
        if (cmd === 'get_project_import_history') {
          return Promise.resolve([])
        }
        if (cmd === 'get_home_directory') {
          return Promise.resolve('/Users/test')
        }
        return Promise.resolve([])
      })

      render(
        <NotificationProvider>
          <Projects initialSelectedProjectId="1" />
        </NotificationProvider>
      )

      await waitFor(() => {
        expect(screen.getByText(/No backup destinations configured/)).toBeTruthy()
      })
    })

    it('queues backup when destination clicked', async () => {
      const project = createMockProject({ id: 'proj-1', name: 'Test Project' })
      const destination = createMockBackupDestination()

      localStorage.setItem('backup_destinations', JSON.stringify([destination]))

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_project') {
          return Promise.resolve(project)
        }
        if (cmd === 'get_project_import_history') {
          return Promise.resolve([])
        }
        if (cmd === 'get_home_directory') {
          return Promise.resolve('/Users/test')
        }
        if (cmd === 'queue_backup') {
          return Promise.resolve()
        }
        return Promise.resolve([])
      })

      const user = userEvent.setup()

      render(
        <NotificationProvider>
          <Projects initialSelectedProjectId="proj-1" />
        </NotificationProvider>
      )

      await waitFor(() => {
        expect(screen.getByText('External Drive')).toBeTruthy()
      })

      await user.click(screen.getByText('External Drive'))

      expect(mockInvoke).toHaveBeenCalledWith('queue_backup', {
        destinationId: 'dest-1',
        destinationName: 'External Drive',
        destinationPath: '/Volumes/Backup',
        projectId: 'proj-1',
        projectName: 'Test Project',
        sourcePath: '/path/to/project',
      })
    })

    it('filters out disabled backup destinations', async () => {
      const project = createMockProject()
      const destinations = [
        createMockBackupDestination({ enabled: true, name: 'Enabled Drive' }),
        createMockBackupDestination({
          enabled: false,
          id: 'dest-2',
          name: 'Disabled Drive',
        }),
      ]

      localStorage.setItem('backup_destinations', JSON.stringify(destinations))

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_project') {
          return Promise.resolve(project)
        }
        if (cmd === 'get_project_import_history') {
          return Promise.resolve([])
        }
        if (cmd === 'get_home_directory') {
          return Promise.resolve('/Users/test')
        }
        return Promise.resolve([])
      })

      render(
        <NotificationProvider>
          <Projects initialSelectedProjectId="1" />
        </NotificationProvider>
      )

      await waitFor(() => {
        expect(screen.getByText('Enabled Drive')).toBeTruthy()
        expect(screen.queryByText('Disabled Drive')).toBeNull()
      })
    })
  })

  describe('deadline Editing', () => {
    it('shows deadline in detail view', async () => {
      const project = createMockProject({ deadline: '2024-12-25' })

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_project') {
          return Promise.resolve(project)
        }
        if (cmd === 'get_project_import_history') {
          return Promise.resolve([])
        }
        if (cmd === 'get_home_directory') {
          return Promise.resolve('/Users/test')
        }
        return Promise.resolve([])
      })

      render(
        <NotificationProvider>
          <Projects initialSelectedProjectId="1" />
        </NotificationProvider>
      )

      await waitFor(() => {
        expect(screen.getByText('Dec 25, 2024')).toBeTruthy()
      })
    })

    it('shows Not set when no deadline', async () => {
      const project = createMockProject({ deadline: undefined })

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_project') {
          return Promise.resolve(project)
        }
        if (cmd === 'get_project_import_history') {
          return Promise.resolve([])
        }
        if (cmd === 'get_home_directory') {
          return Promise.resolve('/Users/test')
        }
        return Promise.resolve([])
      })

      render(
        <NotificationProvider>
          <Projects initialSelectedProjectId="1" />
        </NotificationProvider>
      )

      await waitFor(() => {
        expect(screen.getByText('Not set')).toBeTruthy()
      })
    })

    it('shows overdue text for past deadlines in detail view', async () => {
      const project = createMockProject({ deadline: '2020-01-01' })

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_project') {
          return Promise.resolve(project)
        }
        if (cmd === 'get_project_import_history') {
          return Promise.resolve([])
        }
        if (cmd === 'get_home_directory') {
          return Promise.resolve('/Users/test')
        }
        return Promise.resolve([])
      })

      render(
        <NotificationProvider>
          <Projects initialSelectedProjectId="1" />
        </NotificationProvider>
      )

      await waitFor(() => {
        const elements = screen.getAllByText('Jan 1, 2020')
        expect(elements.length).toBeGreaterThan(0)
        const deadlineElement = elements[0]
        expect(deadlineElement.className).toContain('text-overdue')
      })
    })
  })

  describe('import History', () => {
    it('displays import history when available', async () => {
      const project = createMockProject()
      const history: ImportHistory[] = [
        {
          completedAt: '2024-01-15T10:30:00Z',
          destinationPath: '/path/to/project/RAW',
          errorMessage: undefined,
          filesCopied: 100,
          filesSkipped: 5,
          id: 'import-1',
          photosCopied: 80,
          projectId: '1',
          projectName: 'Test Project',
          sourcePath: '/Volumes/SD1',
          startedAt: '2024-01-15T10:00:00Z',
          status: 'success',
          totalBytes: 1_073_741_824,
          videosCopied: 20,
        },
      ]

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_project') {
          return Promise.resolve(project)
        }
        if (cmd === 'get_project_import_history') {
          return Promise.resolve(history)
        }
        if (cmd === 'get_home_directory') {
          return Promise.resolve('/Users/test')
        }
        return Promise.resolve([])
      })

      render(
        <NotificationProvider>
          <Projects initialSelectedProjectId="1" />
        </NotificationProvider>
      )

      await waitFor(() => {
        expect(screen.getByText(/Photos Imported:/)).toBeTruthy()
        expect(screen.getByText('80')).toBeTruthy()
        expect(screen.getByText(/Videos Imported:/)).toBeTruthy()
        expect(screen.getByText('20')).toBeTruthy()
      })
    })
  })

  describe('keyboard Shortcuts', () => {
    it('closes detail view when Escape key pressed', async () => {
      const project = createMockProject()

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_project') {
          return Promise.resolve(project)
        }
        if (cmd === 'get_project_import_history') {
          return Promise.resolve([])
        }
        if (cmd === 'get_home_directory') {
          return Promise.resolve('/Users/test')
        }
        return Promise.resolve([])
      })

      render(
        <NotificationProvider>
          <Projects initialSelectedProjectId="1" />
        </NotificationProvider>
      )

      await waitFor(() => {
        expect(screen.getByText('← Back')).toBeTruthy()
      })

      fireEvent.keyDown(window, { key: 'Escape' })

      await waitFor(() => {
        expect(screen.queryByText('← Back')).toBeNull()
      })
    })

    it('opens create project dialog when Cmd+N pressed in list view', async () => {
      mockInvoke.mockResolvedValue([])

      render(
        <NotificationProvider>
          <Projects />
        </NotificationProvider>
      )

      await waitFor(() => {
        expect(screen.getByText('Projects')).toBeTruthy()
      })

      fireEvent.keyDown(window, { key: 'n', metaKey: true })

      await waitFor(() => {
        expect(screen.getByText('Create New Project')).toBeTruthy()
      })
    })
  })
})
