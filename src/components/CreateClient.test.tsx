import { beforeEach, describe, expect, it, vi } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import { userEvent } from '@testing-library/user-event'
import { invoke } from '@tauri-apps/api/core'
import { CreateClient } from './CreateClient'
import type { Client } from '../types'

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}))

const mockInvoke = vi.mocked(invoke)

const mockClient: Client = {
  createdAt: '2025-01-01T00:00:00Z',
  id: 'c1',
  name: 'Alice Smith',
  status: 'active',
  updatedAt: '2025-01-01T00:00:00Z',
}

describe('CreateClient', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('renders all fields', () => {
    render(<CreateClient />)
    expect(screen.getByLabelText(/Name/)).toBeTruthy()
    expect(screen.getByLabelText(/Email/)).toBeTruthy()
    expect(screen.getByLabelText(/Phone/)).toBeTruthy()
    expect(screen.getByLabelText(/Notes/)).toBeTruthy()
    expect(screen.getByText('Create Client')).toBeTruthy()
  })

  it('shows cancel button when onCancel provided', () => {
    render(<CreateClient onCancel={vi.fn()} />)
    expect(screen.getByText('Cancel')).toBeTruthy()
  })

  it('hides cancel button when onCancel not provided', () => {
    render(<CreateClient />)
    expect(screen.queryByText('Cancel')).toBeNull()
  })

  it('marks optional fields as optional', () => {
    render(<CreateClient />)
    const optionals = screen.getAllByText('(optional)')
    expect(optionals.length).toBeGreaterThanOrEqual(3)
  })

  it('calls onClientCreated with created client on submit', async () => {
    mockInvoke.mockResolvedValue(mockClient)
    const onClientCreated = vi.fn()
    const user = userEvent.setup()

    render(<CreateClient onClientCreated={onClientCreated} />)
    await user.type(screen.getByLabelText(/Name/), 'Alice Smith')
    await user.click(screen.getByText('Create Client'))

    await waitFor(() => {
      expect(onClientCreated).toHaveBeenCalledWith(mockClient)
    })
  })

  it('invokes create_client with correct args', async () => {
    mockInvoke.mockResolvedValue(mockClient)
    const user = userEvent.setup()

    render(<CreateClient />)
    await user.type(screen.getByLabelText(/Name/), 'Alice Smith')
    await user.type(screen.getByLabelText(/Email/), 'alice@example.com')
    await user.type(screen.getByLabelText(/Phone/), '+1 555 0100')
    await user.click(screen.getByText('Create Client'))

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('create_client', {
        email: 'alice@example.com',
        name: 'Alice Smith',
        notes: null,
        phone: '+1 555 0100',
      })
    })
  })

  it('shows error on invalid email format', async () => {
    const user = userEvent.setup()

    render(<CreateClient />)
    await user.type(screen.getByLabelText(/Name/), 'Alice Smith')
    await user.type(screen.getByLabelText(/Email/), 'notanemail')
    await user.click(screen.getByText('Create Client'))

    await waitFor(() => {
      expect(screen.getByText('Invalid email format')).toBeTruthy()
    })
    expect(mockInvoke).not.toHaveBeenCalled()
  })

  it('shows error message on invoke failure', async () => {
    mockInvoke.mockRejectedValue(new Error("A client named 'Alice Smith' already exists"))
    const user = userEvent.setup()

    render(<CreateClient />)
    await user.type(screen.getByLabelText(/Name/), 'Alice Smith')
    await user.click(screen.getByText('Create Client'))

    await waitFor(() => {
      expect(screen.getByText("A client named 'Alice Smith' already exists")).toBeTruthy()
    })
  })

  it('shows loading state during submit', async () => {
    mockInvoke.mockReturnValue(new Promise(() => {}))
    const user = userEvent.setup()

    render(<CreateClient />)
    await user.type(screen.getByLabelText(/Name/), 'Alice Smith')
    await user.click(screen.getByText('Create Client'))

    await waitFor(() => {
      expect(screen.getByText('Creating...')).toBeTruthy()
    })
  })

  it('prevents submit when name is empty', async () => {
    const user = userEvent.setup()
    render(<CreateClient />)
    await user.click(screen.getByText('Create Client'))
    expect(mockInvoke).not.toHaveBeenCalled()
  })

  it('calls onCancel when cancel clicked', async () => {
    const onCancel = vi.fn()
    const user = userEvent.setup()
    render(<CreateClient onCancel={onCancel} />)
    await user.click(screen.getByText('Cancel'))
    expect(onCancel).toHaveBeenCalledTimes(1)
  })
})
