import { beforeEach, describe, expect, it, vi } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import { userEvent } from '@testing-library/user-event'
import { Settings } from './Settings'
import { NotificationProvider } from '../contexts/NotificationContext'
import { open } from '@tauri-apps/plugin-dialog'

// Mock Tauri API
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockResolvedValue([]),
}))

vi.mock('@tauri-apps/plugin-dialog', () => ({
  open: vi.fn(),
}))

const mockOpen = vi.mocked(open)

describe('settings', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    localStorage.clear()
  })

  it('renders without crashing', () => {
    render(
      <NotificationProvider>
        <Settings />
      </NotificationProvider>
    )

    expect(screen.getByText('Settings')).toBeTruthy()
  })

  it('displays appearance section', () => {
    render(
      <NotificationProvider>
        <Settings />
      </NotificationProvider>
    )

    expect(screen.getByText('Appearance')).toBeTruthy()
    expect(screen.getByText(/Theme/)).toBeTruthy()
  })

  it('displays backup destinations section', () => {
    render(
      <NotificationProvider>
        <Settings />
      </NotificationProvider>
    )

    expect(screen.getByText('Backup Destinations')).toBeTruthy()
  })

  it('adds backup destination', async () => {
    mockOpen.mockResolvedValue('/path/to/backup')
    const user = userEvent.setup()

    render(
      <NotificationProvider>
        <Settings />
      </NotificationProvider>
    )

    const input = screen.getAllByPlaceholderText(/Destination name/)[0]
    await user.type(input, 'External Drive')

    const addButton = screen.getAllByText('Add Destination')[0]
    await user.click(addButton)

    await waitFor(() => {
      expect(screen.getByText('External Drive')).toBeTruthy()
    })
  })

  it('does not add destination with empty name', async () => {
    const consoleWarn = vi.spyOn(console, 'warn').mockImplementation(() => {})
    const user = userEvent.setup()

    render(
      <NotificationProvider>
        <Settings />
      </NotificationProvider>
    )

    const addButton = screen.getAllByText('Add Destination')[0]
    await user.click(addButton)

    expect(mockOpen).not.toHaveBeenCalled()
    expect(consoleWarn).toHaveBeenCalledWith('Destination name is required')
    consoleWarn.mockRestore()
  })

  it('toggles backup destination', async () => {
    localStorage.setItem(
      'backup_destinations',
      JSON.stringify([
        {
          id: '1',
          name: 'Test Dest',
          path: '/test/path',
          enabled: true,
          createdAt: '2025-01-01',
        },
      ])
    )
    const user = userEvent.setup()

    render(
      <NotificationProvider>
        <Settings />
      </NotificationProvider>
    )

    await waitFor(() => expect(screen.getByText('Test Dest')).toBeTruthy())

    const toggle = screen.getAllByRole('checkbox')[0]
    await user.click(toggle)

    const stored = JSON.parse(localStorage.getItem('backup_destinations') || '[]')
    expect(stored[0].enabled).toBe(false)
  })

  it('removes backup destination', async () => {
    localStorage.setItem(
      'backup_destinations',
      JSON.stringify([
        {
          id: '1',
          name: 'Test Dest',
          path: '/test/path',
          enabled: true,
          createdAt: '2025-01-01',
        },
      ])
    )
    const user = userEvent.setup()

    render(
      <NotificationProvider>
        <Settings />
      </NotificationProvider>
    )

    await waitFor(() => expect(screen.getByText('Test Dest')).toBeTruthy())

    const removeButton = screen.getByText('Remove')
    await user.click(removeButton)

    await waitFor(() => {
      const stored = JSON.parse(localStorage.getItem('backup_destinations') || '[]')
      expect(stored).toHaveLength(0)
    })
  })

  it('adds delivery destination', async () => {
    mockOpen.mockResolvedValue('/path/to/delivery')
    const user = userEvent.setup()

    render(
      <NotificationProvider>
        <Settings />
      </NotificationProvider>
    )

    const inputs = screen.getAllByPlaceholderText(/Destination name/)
    await user.type(inputs[1], 'Client Portal')

    const addButtons = screen.getAllByText('Add Destination')
    await user.click(addButtons[1])

    await waitFor(() => {
      expect(screen.getByText('Client Portal')).toBeTruthy()
    })
  })

  it('adds delivery destination with enter key', async () => {
    mockOpen.mockResolvedValue('/path/to/delivery')
    const user = userEvent.setup()

    render(
      <NotificationProvider>
        <Settings />
      </NotificationProvider>
    )

    const inputs = screen.getAllByPlaceholderText(/Destination name/)
    await user.type(inputs[1], 'Client Portal{Enter}')

    await waitFor(() => {
      expect(screen.getByText('Client Portal')).toBeTruthy()
    })
  })

  it('selects default import location', async () => {
    mockOpen.mockResolvedValue('/path/to/imports')
    const user = userEvent.setup()

    render(
      <NotificationProvider>
        <Settings />
      </NotificationProvider>
    )

    const buttons = screen.getAllByText(/Location/)
    const importButton = buttons.find((btn) => btn.textContent === 'Select Location')
    await user.click(importButton!)

    await waitFor(() => {
      expect(localStorage.getItem('default_import_location')).toBe('/path/to/imports')
    })
  })

  it('selects archive location', async () => {
    mockOpen.mockResolvedValue('/path/to/archive')
    const user = userEvent.setup()

    render(
      <NotificationProvider>
        <Settings />
      </NotificationProvider>
    )

    const section = screen.getByText('Archive Location').closest('.card-section')
    const button = section?.querySelector('button')
    await user.click(button!)

    await waitFor(() => {
      expect(localStorage.getItem('archive_location')).toBe('/path/to/archive')
    })
  })

  it('handles dialog cancellation gracefully', async () => {
    mockOpen.mockResolvedValue(null)
    const user = userEvent.setup()

    render(
      <NotificationProvider>
        <Settings />
      </NotificationProvider>
    )

    const input = screen.getAllByPlaceholderText(/Destination name/)[0]
    await user.type(input, 'Test')

    const addButton = screen.getAllByText('Add Destination')[0]
    await user.click(addButton)

    await waitFor(() => {
      const stored = JSON.parse(localStorage.getItem('backup_destinations') || '[]')
      expect(stored).toHaveLength(0)
    })
  })

  it('changes folder template', async () => {
    const user = userEvent.setup()

    render(
      <NotificationProvider>
        <Settings />
      </NotificationProvider>
    )

    const input = screen.getByPlaceholderText('{YYYY}-{MM}-{DD}_{ClientName}_{Type}')
    await user.clear(input)
    await user.type(input, 'CustomFolder')

    await waitFor(() => {
      expect(localStorage.getItem('folder_template')).toBe('CustomFolder')
    })
  })

  it('changes file rename template', async () => {
    const user = userEvent.setup()

    render(
      <NotificationProvider>
        <Settings />
      </NotificationProvider>
    )

    const input = screen.getByPlaceholderText('{original}')
    await user.clear(input)
    await user.type(input, 'CustomName_001')

    await waitFor(() => {
      expect(localStorage.getItem('file_rename_template')).toBe('CustomName_001')
    })
  })

  it('shows correct preview for original file template', () => {
    render(
      <NotificationProvider>
        <Settings />
      </NotificationProvider>
    )

    expect(screen.getByText(/IMG_1234.jpg \(unchanged\)/)).toBeTruthy()
  })

  it('shows correct preview for custom file template', async () => {
    const user = userEvent.setup()

    render(
      <NotificationProvider>
        <Settings />
      </NotificationProvider>
    )

    const input = screen.getByPlaceholderText('{original}')
    await user.clear(input)
    await user.type(input, 'CustomName')

    await waitFor(() => {
      expect(screen.getByText(/CustomName_001.jpg/)).toBeTruthy()
    })
  })

  it('resets templates to defaults', async () => {
    localStorage.setItem('folder_template', 'custom_folder')
    localStorage.setItem('file_rename_template', 'custom_file')
    const user = userEvent.setup()

    render(
      <NotificationProvider>
        <Settings />
      </NotificationProvider>
    )

    const resetButton = screen.getByText('Reset to Defaults')
    await user.click(resetButton)

    await waitFor(() => {
      expect(localStorage.getItem('folder_template')).toBe(
        '{YYYY}-{MM}-{DD}_{ClientName}_{Type}'
      )
      expect(localStorage.getItem('file_rename_template')).toBe('{original}')
    })
  })

  it('toggles auto eject', async () => {
    const user = userEvent.setup()

    render(
      <NotificationProvider>
        <Settings />
      </NotificationProvider>
    )

    const checkbox = screen.getByLabelText(/Auto-eject/)
    await user.click(checkbox)

    await waitFor(() => {
      expect(localStorage.getItem('auto_eject')).toBe('true')
    })

    await user.click(checkbox)

    await waitFor(() => {
      expect(localStorage.getItem('auto_eject')).toBe('false')
    })
  })

  it('loads existing settings on mount', () => {
    localStorage.setItem('default_import_location', '/custom/import')
    localStorage.setItem('archive_location', '/custom/archive')
    localStorage.setItem('folder_template', 'custom_folder')
    localStorage.setItem('file_rename_template', 'custom_file')
    localStorage.setItem('auto_eject', 'true')

    render(
      <NotificationProvider>
        <Settings />
      </NotificationProvider>
    )

    expect(screen.getByText('/custom/import')).toBeTruthy()
    expect(screen.getByText('/custom/archive')).toBeTruthy()
    expect((screen.getByPlaceholderText('{YYYY}-{MM}-{DD}_{ClientName}_{Type}') as HTMLInputElement).value).toBe('custom_folder')
    expect((screen.getByPlaceholderText('{original}') as HTMLInputElement).value).toBe('custom_file')
  })
})
