import { beforeEach, describe, expect, it, vi } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import { userEvent } from '@testing-library/user-event'
import { CommandPalette } from './CommandPalette'
import { invoke } from '@tauri-apps/api/core'
import type { Project, ProjectStatus } from '../types'

// Mock Tauri API
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}))

const mockInvoke = vi.mocked(invoke)

// Mock scrollIntoView
Element.prototype.scrollIntoView = vi.fn()

const mockProjects: Project[] = [
  {
    id: '1',
    name: 'Wedding Photos',
    clientName: 'John Smith',
    shootType: 'Wedding',
    date: '2025-01-15',
    status: 'New' as ProjectStatus,
    folderPath: '/path/to/wedding',
    createdAt: '2025-01-01',
    updatedAt: '2025-01-01',
  },
  {
    id: '2',
    name: 'Corporate Event',
    clientName: 'Acme Corp',
    shootType: 'Event',
    date: '2025-02-20',
    status: 'Editing' as ProjectStatus,
    folderPath: '/path/to/corporate',
    createdAt: '2025-01-02',
    updatedAt: '2025-01-02',
  },
  {
    id: '3',
    name: 'Portrait Session',
    clientName: 'Jane Doe',
    shootType: 'Portrait',
    date: '2025-03-10',
    status: 'Delivered' as ProjectStatus,
    folderPath: '/path/to/portrait',
    createdAt: '2025-01-03',
    updatedAt: '2025-01-03',
  },
  {
    id: '4',
    name: 'Product Shoot',
    clientName: 'Shop Inc',
    shootType: 'Product',
    date: '2025-04-05',
    status: 'Archived' as ProjectStatus,
    folderPath: '/path/to/product',
    createdAt: '2025-01-04',
    updatedAt: '2025-01-04',
  },
  {
    id: '5',
    name: 'Fashion Editorial',
    clientName: 'Style Magazine',
    shootType: 'Fashion',
    date: '2025-05-12',
    status: 'Importing' as ProjectStatus,
    folderPath: '/path/to/fashion',
    createdAt: '2025-01-05',
    updatedAt: '2025-01-05',
  },
]

describe('commandPalette', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockInvoke.mockResolvedValue([])
  })

  it('renders nothing when not open', () => {
    const { container } = render(
      <CommandPalette isOpen={false} onClose={vi.fn()} onSelectProject={vi.fn()} />
    )

    expect(container.firstChild).toBeNull()
  })

  it('renders when open', () => {
    render(<CommandPalette isOpen onClose={vi.fn()} onSelectProject={vi.fn()} />)

    expect(screen.getByPlaceholderText(/Search/)).toBeTruthy()
  })

  it('calls onClose when overlay clicked', async () => {
    const onClose = vi.fn()
    const user = userEvent.setup()

    render(<CommandPalette isOpen onClose={onClose} onSelectProject={vi.fn()} />)

    const overlay = document.querySelector('.command-palette-overlay')
    await user.click(overlay!)

    expect(onClose).toHaveBeenCalledTimes(1)
  })

  it('loads and displays projects', async () => {
    mockInvoke.mockResolvedValue(mockProjects)
    render(<CommandPalette isOpen onClose={vi.fn()} onSelectProject={vi.fn()} />)

    await waitFor(() => {
      expect(screen.getByText('Wedding Photos')).toBeTruthy()
      expect(screen.getByText('Corporate Event')).toBeTruthy()
      expect(screen.getByText('Portrait Session')).toBeTruthy()
    })
  })

  it('filters projects by name', async () => {
    mockInvoke.mockResolvedValue(mockProjects)
    const user = userEvent.setup()
    render(<CommandPalette isOpen onClose={vi.fn()} onSelectProject={vi.fn()} />)

    await waitFor(() => expect(screen.getByText('Wedding Photos')).toBeTruthy())

    const input = screen.getByPlaceholderText(/Search/)
    await user.type(input, 'wedding')

    expect(screen.getByText('Wedding Photos')).toBeTruthy()
    expect(screen.queryByText('Corporate Event')).toBeNull()
  })

  it('filters projects by client name', async () => {
    mockInvoke.mockResolvedValue(mockProjects)
    const user = userEvent.setup()
    render(<CommandPalette isOpen onClose={vi.fn()} onSelectProject={vi.fn()} />)

    await waitFor(() => expect(screen.getByText('Wedding Photos')).toBeTruthy())

    const input = screen.getByPlaceholderText(/Search/)
    await user.type(input, 'acme')

    expect(screen.getByText('Corporate Event')).toBeTruthy()
    expect(screen.queryByText('Wedding Photos')).toBeNull()
  })

  it('filters projects by shoot type', async () => {
    mockInvoke.mockResolvedValue(mockProjects)
    const user = userEvent.setup()
    render(<CommandPalette isOpen onClose={vi.fn()} onSelectProject={vi.fn()} />)

    await waitFor(() => expect(screen.getByText('Portrait Session')).toBeTruthy())

    const input = screen.getByPlaceholderText(/Search/)
    await user.type(input, 'portrait')

    expect(screen.getByText('Portrait Session')).toBeTruthy()
    expect(screen.queryByText('Wedding Photos')).toBeNull()
  })

  it('filters projects by date', async () => {
    mockInvoke.mockResolvedValue(mockProjects)
    const user = userEvent.setup()
    render(<CommandPalette isOpen onClose={vi.fn()} onSelectProject={vi.fn()} />)

    await waitFor(() => expect(screen.getByText('Wedding Photos')).toBeTruthy())

    const input = screen.getByPlaceholderText(/Search/)
    await user.type(input, '2025-01-15')

    expect(screen.getByText('Wedding Photos')).toBeTruthy()
    expect(screen.queryByText('Corporate Event')).toBeNull()
  })

  it('shows empty state when no projects match', async () => {
    mockInvoke.mockResolvedValue(mockProjects)
    const user = userEvent.setup()
    render(<CommandPalette isOpen onClose={vi.fn()} onSelectProject={vi.fn()} />)

    await waitFor(() => expect(screen.getByText('Wedding Photos')).toBeTruthy())

    const input = screen.getByPlaceholderText(/Search/)
    await user.type(input, 'nonexistent')

    expect(screen.getByText('No projects found')).toBeTruthy()
  })

  it('shows empty state when no projects available', () => {
    mockInvoke.mockResolvedValue([])
    render(<CommandPalette isOpen onClose={vi.fn()} onSelectProject={vi.fn()} />)

    expect(screen.getByText('No projects available')).toBeTruthy()
  })

  it('navigates with arrow down key', async () => {
    mockInvoke.mockResolvedValue(mockProjects)
    const user = userEvent.setup()
    render(<CommandPalette isOpen onClose={vi.fn()} onSelectProject={vi.fn()} />)

    await waitFor(() => expect(screen.getByText('Wedding Photos')).toBeTruthy())

    await user.keyboard('{ArrowDown}')

    const items = document.querySelectorAll('.command-palette-item')
    expect(items[1].classList.contains('selected')).toBe(true)
  })

  it('navigates with arrow up key', async () => {
    mockInvoke.mockResolvedValue(mockProjects)
    const user = userEvent.setup()
    render(<CommandPalette isOpen onClose={vi.fn()} onSelectProject={vi.fn()} />)

    await waitFor(() => expect(screen.getByText('Wedding Photos')).toBeTruthy())

    await user.keyboard('{ArrowDown}')
    await user.keyboard('{ArrowDown}')
    await user.keyboard('{ArrowUp}')

    const items = document.querySelectorAll('.command-palette-item')
    expect(items[1].classList.contains('selected')).toBe(true)
  })

  it('does not navigate below first item with arrow up', async () => {
    mockInvoke.mockResolvedValue(mockProjects)
    const user = userEvent.setup()
    render(<CommandPalette isOpen onClose={vi.fn()} onSelectProject={vi.fn()} />)

    await waitFor(() => expect(screen.getByText('Wedding Photos')).toBeTruthy())

    await user.keyboard('{ArrowUp}')

    const items = document.querySelectorAll('.command-palette-item')
    expect(items[0].classList.contains('selected')).toBe(true)
  })

  it('does not navigate beyond last item with arrow down', async () => {
    mockInvoke.mockResolvedValue(mockProjects.slice(0, 2))
    const user = userEvent.setup()
    render(<CommandPalette isOpen onClose={vi.fn()} onSelectProject={vi.fn()} />)

    await waitFor(() => expect(screen.getByText('Wedding Photos')).toBeTruthy())

    await user.keyboard('{ArrowDown}')
    await user.keyboard('{ArrowDown}')
    await user.keyboard('{ArrowDown}')

    const items = document.querySelectorAll('.command-palette-item')
    expect(items[1].classList.contains('selected')).toBe(true)
  })

  it('selects project on enter key', async () => {
    mockInvoke.mockResolvedValue(mockProjects)
    const onSelectProject = vi.fn()
    const onClose = vi.fn()
    const user = userEvent.setup()
    render(<CommandPalette isOpen onClose={onClose} onSelectProject={onSelectProject} />)

    await waitFor(() => expect(screen.getByText('Wedding Photos')).toBeTruthy())

    await user.keyboard('{ArrowDown}')
    await user.keyboard('{Enter}')

    expect(onSelectProject).toHaveBeenCalledWith('2')
    expect(onClose).toHaveBeenCalled()
  })

  it('selects project on click', async () => {
    mockInvoke.mockResolvedValue(mockProjects)
    const onSelectProject = vi.fn()
    const onClose = vi.fn()
    const user = userEvent.setup()
    render(<CommandPalette isOpen onClose={onClose} onSelectProject={onSelectProject} />)

    await waitFor(() => expect(screen.getByText('Wedding Photos')).toBeTruthy())

    await user.click(screen.getByText('Wedding Photos'))

    expect(onSelectProject).toHaveBeenCalledWith('1')
    expect(onClose).toHaveBeenCalled()
  })

  it('updates selected index on mouse enter', async () => {
    mockInvoke.mockResolvedValue(mockProjects)
    const user = userEvent.setup()
    render(<CommandPalette isOpen onClose={vi.fn()} onSelectProject={vi.fn()} />)

    await waitFor(() => expect(screen.getByText('Corporate Event')).toBeTruthy())

    const item = screen.getByText('Corporate Event').closest('.command-palette-item')
    await user.hover(item!)

    expect(item?.classList.contains('selected')).toBe(true)
  })

  it('closes on escape key', async () => {
    mockInvoke.mockResolvedValue(mockProjects)
    const onClose = vi.fn()
    const user = userEvent.setup()
    render(<CommandPalette isOpen onClose={onClose} onSelectProject={vi.fn()} />)

    await waitFor(() => expect(screen.getByText('Wedding Photos')).toBeTruthy())

    await user.keyboard('{Escape}')

    expect(onClose).toHaveBeenCalled()
  })

  it('resets search and selection when opened', async () => {
    mockInvoke.mockResolvedValue(mockProjects)
    const user = userEvent.setup()
    const { rerender } = render(
      <CommandPalette isOpen onClose={vi.fn()} onSelectProject={vi.fn()} />
    )

    await waitFor(() => expect(screen.getByText('Wedding Photos')).toBeTruthy())

    const input = screen.getByPlaceholderText(/Search/)
    await user.type(input, 'test')
    await user.keyboard('{ArrowDown}')

    rerender(<CommandPalette isOpen={false} onClose={vi.fn()} onSelectProject={vi.fn()} />)
    rerender(<CommandPalette isOpen onClose={vi.fn()} onSelectProject={vi.fn()} />)

    await waitFor(() => {
      const newInput = screen.getByPlaceholderText(/Search/) as HTMLInputElement
      expect(newInput.value).toBe('')
    })
  })

  it('displays all project status colors', async () => {
    mockInvoke.mockResolvedValue(mockProjects)
    render(<CommandPalette isOpen onClose={vi.fn()} onSelectProject={vi.fn()} />)

    await waitFor(() => {
      expect(screen.getByText('Wedding Photos')).toBeTruthy()
    })

    const statuses = document.querySelectorAll('.project-status')
    expect(statuses[0].classList.contains('status-new')).toBe(true)
    expect(statuses[1].classList.contains('status-editing')).toBe(true)
    expect(statuses[2].classList.contains('status-delivered')).toBe(true)
    expect(statuses[3].classList.contains('status-archived')).toBe(true)
    expect(statuses[4].classList.contains('status-importing')).toBe(true)
  })

  it('does not prevent default for unhandled keys', async () => {
    mockInvoke.mockResolvedValue(mockProjects)
    const user = userEvent.setup()
    render(<CommandPalette isOpen onClose={vi.fn()} onSelectProject={vi.fn()} />)

    await waitFor(() => expect(screen.getByText('Wedding Photos')).toBeTruthy())

    const input = screen.getByPlaceholderText(/Search/)
    await user.type(input, 'a')

    expect((input as HTMLInputElement).value).toBe('a')
  })
})
