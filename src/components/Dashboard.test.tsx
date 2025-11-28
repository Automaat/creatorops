import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { Dashboard } from './Dashboard'
import { Project, ProjectStatus } from '../types'
import { invoke } from '@tauri-apps/api/core'

// Mock Tauri API
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}))

const mockInvoke = vi.mocked(invoke)
// Mock CreateProject component
vi.mock('./CreateProject', () => ({
  CreateProject: ({
    onProjectCreated,
    onCancel,
  }: {
    onProjectCreated?: (project: Partial<Project>) => void
    onCancel?: () => void
  }) => (
    <div data-testid="create-project-form">
      <button
        onClick={() =>
          onProjectCreated?.({ id: 'new-id', name: 'New Project', status: ProjectStatus.Editing })
        }
      >
        Create
      </button>
      <button onClick={onCancel}>Cancel</button>
    </div>
  ),
}))

describe('Dashboard', () => {
  beforeEach(() => {
    localStorage.clear()
    vi.clearAllMocks()
  })

  afterEach(() => {
    localStorage.clear()
  })

  const mockProjects: Project[] = [
    {
      id: '1',
      name: 'Wedding Photos',
      status: ProjectStatus.Editing,
      deadline: '2025-12-01',
      clientName: 'John Doe',
      date: '2025-11-15',
      shootType: 'Wedding',
      folderPath: '/path/to/wedding',
      createdAt: '2025-11-01',
      updatedAt: '2025-11-01',
    },
    {
      id: '2',
      name: 'Corporate Event',
      status: ProjectStatus.Importing,
      clientName: 'Acme Corp',
      date: '2025-11-20',
      shootType: 'Event',
      folderPath: '/path/to/event',
      createdAt: '2025-11-02',
      updatedAt: '2025-11-02',
    },
  ]

  it('shows loading state initially', () => {
    mockInvoke.mockReturnValue(new Promise(() => {}))

    render(<Dashboard />)
    expect(screen.getByText('Loading...')).toBeTruthy()
  })

  it('loads and displays projects', async () => {
    mockInvoke.mockResolvedValue(mockProjects)

    render(<Dashboard />)

    await waitFor(() => {
      expect(screen.getByText('Wedding Photos')).toBeTruthy()
      expect(screen.getByText('Corporate Event')).toBeTruthy()
    })
  })

  it('displays project status badges', async () => {
    mockInvoke.mockResolvedValue(mockProjects)

    render(<Dashboard />)

    await waitFor(() => {
      expect(screen.getByText('Editing')).toBeTruthy()
      expect(screen.getByText('Importing')).toBeTruthy()
    })
  })

  it('displays empty state when no projects', async () => {
    mockInvoke.mockResolvedValue([])

    render(<Dashboard />)

    await waitFor(() => {
      expect(screen.getByText('No active projects')).toBeTruthy()
    })
  })

  it('filters out archived projects from active list', async () => {
    const projectsWithArchived: Project[] = [
      ...mockProjects,
      {
        id: '3',
        name: 'Archived Project',
        status: ProjectStatus.Archived,
        clientName: 'Old Client',
        date: '2025-10-01',
        shootType: 'Portrait',
        folderPath: '/path/to/archived',
        createdAt: '2025-10-01',
        updatedAt: '2025-10-01',
      },
    ]

    mockInvoke.mockResolvedValue(projectsWithArchived)

    render(<Dashboard />)

    await waitFor(() => {
      expect(screen.queryByText('Archived Project')).toBeNull()
      expect(screen.getByText('Wedding Photos')).toBeTruthy()
    })
  })

  it('displays correct total and active project counts', async () => {
    mockInvoke.mockResolvedValue(mockProjects)

    render(<Dashboard />)

    await waitFor(() => {
      const statValues = screen.getAllByText('2')
      expect(statValues.length).toBeGreaterThanOrEqual(2) // Both total and active should be 2
    })
  })

  it('calls onProjectClick when project is clicked', async () => {
    mockInvoke.mockResolvedValue(mockProjects)

    const onProjectClick = vi.fn()
    const user = userEvent.setup()

    render(<Dashboard onProjectClick={onProjectClick} />)

    await waitFor(() => {
      expect(screen.getByText('Wedding Photos')).toBeTruthy()
    })

    const project = screen.getByText('Wedding Photos')
    await user.click(project)

    expect(onProjectClick).toHaveBeenCalledWith('1')
  })

  it('opens create project dialog when New Project clicked', async () => {
    mockInvoke.mockResolvedValue([])

    const user = userEvent.setup()

    render(<Dashboard />)

    await waitFor(() => {
      expect(screen.getByText('Dashboard')).toBeTruthy()
    })

    const newProjectBtn = screen.getByText('New Project')
    await user.click(newProjectBtn)

    expect(screen.getByTestId('create-project-form')).toBeTruthy()
    expect(screen.getByText('Create New Project')).toBeTruthy()
  })

  it('closes create project dialog on cancel', async () => {
    mockInvoke.mockResolvedValue([])

    const user = userEvent.setup()

    render(<Dashboard />)

    await waitFor(() => {
      expect(screen.getByText('Dashboard')).toBeTruthy()
    })

    const newProjectBtn = screen.getByText('New Project')
    await user.click(newProjectBtn)

    const cancelBtn = screen.getByText('Cancel')
    await user.click(cancelBtn)

    expect(screen.queryByTestId('create-project-form')).toBeNull()
  })

  it('handles load data errors gracefully', async () => {
    const consoleError = console.error
    console.error = vi.fn()

    mockInvoke.mockRejectedValue(new Error('Load failed'))

    render(<Dashboard />)

    await waitFor(() => {
      expect(console.error).toHaveBeenCalledWith(
        'Failed to load dashboard data:',
        expect.any(Error)
      )
    })

    console.error = consoleError
  })

  it('sorts projects by deadline first', async () => {
    const projectsWithDeadlines: Project[] = [
      { ...mockProjects[0], deadline: '2025-12-25' },
      { ...mockProjects[1], deadline: '2025-12-01' },
    ]

    mockInvoke.mockResolvedValue(projectsWithDeadlines)

    render(<Dashboard />)

    await waitFor(() => {
      const projectItems = document.querySelectorAll('.project-list-item h3')
      // Earlier deadline should come first
      expect(projectItems[0].textContent).toBe('Corporate Event') // 2025-12-01
      expect(projectItems[1].textContent).toBe('Wedding Photos') // 2025-12-25
    })
  })
})
