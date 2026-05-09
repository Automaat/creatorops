import { beforeEach, describe, expect, it, vi } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import { userEvent } from '@testing-library/user-event'
import { invoke } from '@tauri-apps/api/core'
import { NotificationProvider } from '../contexts/NotificationContext'
import { Clients } from './Clients'
import type { Client, ClientWithProjects } from '../types'

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}))

const mockInvoke = vi.mocked(invoke)

const mockClients: Client[] = [
  {
    createdAt: '2025-01-01T00:00:00Z',
    email: 'alice@example.com',
    id: 'c1',
    name: 'Alice Smith',
    status: 'active',
    updatedAt: '2025-01-01T00:00:00Z',
  },
  {
    createdAt: '2025-01-01T00:00:00Z',
    id: 'c2',
    name: 'Bob Jones',
    status: 'active',
    updatedAt: '2025-01-01T00:00:00Z',
  },
]

const mockClientDetail: ClientWithProjects = {
  ...mockClients[0],
  projects: [],
}

describe('Clients', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockInvoke.mockResolvedValue(mockClients)
  })

  it('shows loading initially', () => {
    mockInvoke.mockReturnValue(new Promise(() => {}))
    render(
      <NotificationProvider>
        <Clients />
      </NotificationProvider>
    )
    expect(screen.getByText('Loading...')).toBeTruthy()
  })

  it('renders client list after loading', async () => {
    render(
      <NotificationProvider>
        <Clients />
      </NotificationProvider>
    )
    await waitFor(() => {
      expect(screen.getByText('Alice Smith')).toBeTruthy()
      expect(screen.getByText('Bob Jones')).toBeTruthy()
    })
  })

  it('shows empty state when no clients', async () => {
    mockInvoke.mockResolvedValue([])
    render(
      <NotificationProvider>
        <Clients />
      </NotificationProvider>
    )
    await waitFor(() => {
      expect(screen.getByText('No clients yet')).toBeTruthy()
    })
  })

  it('filters clients by search query', async () => {
    const user = userEvent.setup()
    render(
      <NotificationProvider>
        <Clients />
      </NotificationProvider>
    )

    await waitFor(() => screen.getByText('Alice Smith'))

    await user.type(screen.getByPlaceholderText('Search clients...'), 'Alice')

    expect(screen.getByText('Alice Smith')).toBeTruthy()
    expect(screen.queryByText('Bob Jones')).toBeNull()
  })

  it('opens create client dialog', async () => {
    const user = userEvent.setup()
    render(
      <NotificationProvider>
        <Clients />
      </NotificationProvider>
    )

    await waitFor(() => screen.getByText('Alice Smith'))
    await user.click(screen.getByText('Create Client'))

    expect(screen.getByText('Create New Client')).toBeTruthy()
  })

  it('navigates to client detail on click', async () => {
    mockInvoke.mockResolvedValueOnce(mockClients).mockResolvedValueOnce(mockClientDetail)

    const user = userEvent.setup()
    render(
      <NotificationProvider>
        <Clients />
      </NotificationProvider>
    )

    await waitFor(() => screen.getByText('Alice Smith'))
    await user.click(screen.getByText('Alice Smith'))

    await waitFor(() => {
      expect(screen.getByText('← Back')).toBeTruthy()
      expect(screen.getByText('Projects (0)')).toBeTruthy()
    })
  })

  it('returns to list on back button click', async () => {
    mockInvoke
      .mockResolvedValueOnce(mockClients)
      .mockResolvedValueOnce(mockClientDetail)
      .mockResolvedValueOnce(mockClients)

    const user = userEvent.setup()
    render(
      <NotificationProvider>
        <Clients />
      </NotificationProvider>
    )

    await waitFor(() => screen.getByText('Alice Smith'))
    await user.click(screen.getByText('Alice Smith'))
    await waitFor(() => screen.getByText('← Back'))
    await user.click(screen.getByText('← Back'))

    await waitFor(() => {
      expect(screen.queryByText('← Back')).toBeNull()
    })
  })

  it('shows email in client card', async () => {
    render(
      <NotificationProvider>
        <Clients />
      </NotificationProvider>
    )
    await waitFor(() => {
      expect(screen.getByText('alice@example.com')).toBeTruthy()
    })
  })
})
