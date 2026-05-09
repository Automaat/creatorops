import { beforeEach, describe, expect, it, vi } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import { userEvent } from '@testing-library/user-event'
import { invoke } from '@tauri-apps/api/core'
import { ClientSelector } from './ClientSelector'
import type { Client } from '../types'

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}))

const mockInvoke = vi.mocked(invoke)

const mockClients: Client[] = [
  {
    createdAt: '2025-01-01T00:00:00Z',
    id: 'c1',
    name: 'Alice Smith',
    status: 'active',
    updatedAt: '2025-01-01T00:00:00Z',
  },
  {
    createdAt: '2025-01-01T00:00:00Z',
    email: 'bob@example.com',
    id: 'c2',
    name: 'Bob Jones',
    status: 'active',
    updatedAt: '2025-01-01T00:00:00Z',
  },
]

describe('ClientSelector', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockInvoke.mockResolvedValue(mockClients)
  })

  it('renders input field', async () => {
    render(<ClientSelector clientId={null} onChange={vi.fn()} value="" />)
    expect(screen.getByRole('textbox')).toBeTruthy()
  })

  it('loads clients on mount', async () => {
    render(<ClientSelector clientId={null} onChange={vi.fn()} value="" />)
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('list_clients', { includeArchived: false })
    })
  })

  it('shows dropdown with matching clients on input', async () => {
    const user = userEvent.setup()
    render(<ClientSelector clientId={null} onChange={vi.fn()} value="" />)
    await waitFor(() => expect(mockInvoke).toHaveBeenCalled())

    await user.click(screen.getByRole('textbox'))
    await user.type(screen.getByRole('textbox'), 'Ali')

    await waitFor(() => {
      expect(screen.getByText('Alice Smith')).toBeTruthy()
    })
    expect(screen.queryByText('Bob Jones')).toBeNull()
  })

  it('calls onChange with client data on selection', async () => {
    const onChange = vi.fn()
    const user = userEvent.setup()

    render(<ClientSelector clientId={null} onChange={onChange} value="" />)
    await waitFor(() => expect(mockInvoke).toHaveBeenCalled())

    await user.click(screen.getByRole('textbox'))
    await user.type(screen.getByRole('textbox'), 'Alice')

    await waitFor(() => screen.getByText('Alice Smith'))
    await user.click(screen.getByText('Alice Smith'))

    expect(onChange).toHaveBeenCalledWith('Alice Smith', 'c1')
  })

  it('shows create option when query has no exact match', async () => {
    const user = userEvent.setup()
    render(<ClientSelector clientId={null} onChange={vi.fn()} value="" />)
    await waitFor(() => expect(mockInvoke).toHaveBeenCalled())

    await user.click(screen.getByRole('textbox'))
    await user.type(screen.getByRole('textbox'), 'New Client')

    await waitFor(() => {
      expect(screen.getByText(/Create.*New Client/)).toBeTruthy()
    })
  })

  it('shows client email in dropdown', async () => {
    const user = userEvent.setup()
    render(<ClientSelector clientId={null} onChange={vi.fn()} value="" />)
    await waitFor(() => expect(mockInvoke).toHaveBeenCalled())

    await user.click(screen.getByRole('textbox'))
    await user.type(screen.getByRole('textbox'), 'Bob')

    await waitFor(() => {
      expect(screen.getByText('bob@example.com')).toBeTruthy()
    })
  })
})
