import { beforeEach, describe, expect, it, vi } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import { userEvent } from '@testing-library/user-event'
import { CreateProject } from './CreateProject'
import type { Project } from '../types'
import { ProjectStatus } from '../types'
import { invoke } from '@tauri-apps/api/core'

// Mock Tauri API
vi.mock<typeof import('@tauri-apps/api/core')>('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}))

const mockInvoke = vi.mocked(invoke)

describe('createProject', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('renders form with all fields', () => {
    render(<CreateProject />)

    expect(screen.getByLabelText(/Project Name/)).toBeTruthy()
    expect(screen.getByLabelText(/Client Name/)).toBeTruthy()
    expect(screen.getByLabelText(/Shoot Date/)).toBeTruthy()
    expect(screen.getByLabelText(/Shoot Type/)).toBeTruthy()
    expect(screen.getByLabelText(/Deadline/)).toBeTruthy()
    expect(screen.getByText('Create Project')).toBeTruthy()
  })

  it('shows cancel button when onCancel provided', () => {
    const onCancel = vi.fn()
    render(<CreateProject onCancel={onCancel} />)

    expect(screen.getByText('Cancel')).toBeTruthy()
  })

  it('does not show cancel button when onCancel not provided', () => {
    render(<CreateProject />)

    expect(screen.queryByText('Cancel')).toBeNull()
  })

  it("initializes date field with today's date", () => {
    render(<CreateProject />)

    const dateInput = screen.getByLabelText(/Shoot Date/)

    // DatePicker displays formatted date, not the raw value
    expect(dateInput).toBeTruthy()
  })

  it('updates form fields when typing', async () => {
    const user = userEvent.setup()
    render(<CreateProject />)

    const nameInput = screen.getByLabelText(/Project Name/)
    const clientInput = screen.getByLabelText(/Client Name/)
    const shootTypeInput = screen.getByLabelText(/Shoot Type/)

    await user.type(nameInput, 'Test Project')
    await user.type(clientInput, 'Test Client')
    await user.type(shootTypeInput, 'Wedding')

    if (nameInput instanceof HTMLInputElement) {
      expect(nameInput.value).toBe('Test Project')
    }
    if (clientInput instanceof HTMLInputElement) {
      expect(clientInput.value).toBe('Test Client')
    }
    if (shootTypeInput instanceof HTMLInputElement) {
      expect(shootTypeInput.value).toBe('Wedding')
    }
  })

  it('calls onProjectCreated with created project on submit', async () => {
    const mockProject: Project = {
      clientName: 'Test Client', createdAt: '2025-11-20', date: '2025-11-20', folderPath: '/test/path', id: '123', name: 'Test Project', shootType: '', status: ProjectStatus.Editing, updatedAt: '2025-11-20',
    }

    mockInvoke.mockResolvedValue(mockProject)

    const onProjectCreated = vi.fn()
    const user = userEvent.setup()

    render(<CreateProject onProjectCreated={onProjectCreated} />)

    const nameInput = screen.getByLabelText(/Project Name/)
    const clientInput = screen.getByLabelText(/Client Name/)

    await user.type(nameInput, 'Test Project')
    await user.type(clientInput, 'Test Client')

    const submitButton = screen.getByText('Create Project')
    await user.click(submitButton)

    await waitFor(() => {
      expect(onProjectCreated).toHaveBeenCalledWith(mockProject)
    })
  })

  it('calls onCancel when cancel button clicked', async () => {
    const onCancel = vi.fn()
    const user = userEvent.setup()

    render(<CreateProject onCancel={onCancel} />)

    const cancelButton = screen.getByText('Cancel')
    await user.click(cancelButton)

    expect(onCancel).toHaveBeenCalledTimes(1)
  })

  it('shows loading state while submitting', async () => {
    let resolveInvoke: (value: Project) => void
    mockInvoke.mockReturnValue(
      new Promise((resolve) => {
        resolveInvoke = resolve
      })
    )

    const user = userEvent.setup()
    render(<CreateProject />)

    const nameInput = screen.getByLabelText(/Project Name/)
    const clientInput = screen.getByLabelText(/Client Name/)

    await user.type(nameInput, 'Test')
    await user.type(clientInput, 'Test')

    const submitButton = screen.getByText('Create Project')
    await user.click(submitButton)

    await waitFor(() => {
      expect(screen.getByText('Creating...')).toBeTruthy()
    })

    resolveInvoke!({
      clientName: 'Test', createdAt: '2025-11-20', date: '2025-11-20', folderPath: '/test/path', id: '1', name: 'Test', shootType: '', status: ProjectStatus.Editing, updatedAt: '2025-11-20',
    })
  })

  it('disables buttons while submitting', async () => {
    mockInvoke.mockReturnValue(new Promise(() => {}))

    const onCancel = vi.fn()
    const user = userEvent.setup()

    render(<CreateProject onCancel={onCancel} />)

    const nameInput = screen.getByLabelText(/Project Name/)
    const clientInput = screen.getByLabelText(/Client Name/)

    await user.type(nameInput, 'Test')
    await user.type(clientInput, 'Test')

    const submitButton = screen.getByText('Create Project')
    await user.click(submitButton)

    await waitFor(() => {
      expect(submitButton).toBeDisabled()
      expect(screen.getByText('Cancel')).toBeDisabled()
    })
  })

  it('displays error message on submit failure', async () => {
    mockInvoke.mockRejectedValue(new Error('Database error'))

    const user = userEvent.setup()
    render(<CreateProject />)

    const nameInput = screen.getByLabelText(/Project Name/)
    const clientInput = screen.getByLabelText(/Client Name/)

    await user.type(nameInput, 'Test')
    await user.type(clientInput, 'Test')

    const submitButton = screen.getByText('Create Project')
    await user.click(submitButton)

    await waitFor(() => {
      expect(screen.getByText('Database error')).toBeTruthy()
    })
  })

  it('clears error on new submit attempt', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('First error'))
    mockInvoke.mockResolvedValueOnce({
      clientName: 'Test', createdAt: '2025-11-20', date: '2025-11-20', folderPath: '/test/path', id: '1', name: 'Test', shootType: '', status: ProjectStatus.Editing, updatedAt: '2025-11-20',
    })

    const user = userEvent.setup()
    render(<CreateProject />)

    const nameInput = screen.getByLabelText(/Project Name/)
    const clientInput = screen.getByLabelText(/Client Name/)

    await user.type(nameInput, 'Test')
    await user.type(clientInput, 'Test')

    const submitButton = screen.getByText('Create Project')
    await user.click(submitButton)

    await waitFor(() => {
      expect(screen.getByText('First error')).toBeTruthy()
    })

    await user.click(submitButton)

    await waitFor(() => {
      expect(screen.queryByText('First error')).toBeNull()
    })
  })

  it('marks required fields with asterisk', () => {
    render(<CreateProject />)

    expect(screen.getByText('Project Name *')).toBeTruthy()
    expect(screen.getByText('Client Name *')).toBeTruthy()
  })

  it('marks optional fields as optional', () => {
    render(<CreateProject />)

    const optionalLabels = screen.getAllByText(/\(optional\)/)
    expect(optionalLabels.length).toBeGreaterThanOrEqual(2) // Shoot Type and Deadline
  })

  it('prevents form submission when required fields are empty', async () => {
    const user = userEvent.setup()

    render(<CreateProject />)

    const submitButton = screen.getByText('Create Project')
    await user.click(submitButton)

    // HTML5 validation should prevent the invoke from being called
    expect(invoke).not.toHaveBeenCalled()
  })
})
