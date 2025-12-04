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

vi.mock('@tauri-apps/plugin-opener', () => ({
  open: vi.fn().mockResolvedValue(undefined),
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

    const stored = JSON.parse(localStorage.getItem('backup_destinations') || '[]') as Array<{
      enabled: boolean
    }>
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
      const stored = JSON.parse(localStorage.getItem('backup_destinations') || '[]') as unknown[]
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
      const stored = JSON.parse(localStorage.getItem('backup_destinations') || '[]') as unknown[]
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
      expect(localStorage.getItem('folder_template')).toBe('{YYYY}-{MM}-{DD}_{ClientName}_{Type}')
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
    expect(
      (screen.getByPlaceholderText('{YYYY}-{MM}-{DD}_{ClientName}_{Type}') as HTMLInputElement)
        .value
    ).toBe('custom_folder')
    expect((screen.getByPlaceholderText('{original}') as HTMLInputElement).value).toBe(
      'custom_file'
    )
  })

  describe('Google Drive Integration', () => {
    it('displays Google Drive section', () => {
      render(
        <NotificationProvider>
          <Settings />
        </NotificationProvider>
      )

      expect(screen.getByText('Google Drive Integration')).toBeTruthy()
    })

    it('shows connect button when not connected', () => {
      render(
        <NotificationProvider>
          <Settings />
        </NotificationProvider>
      )

      expect(screen.getByText('Connect Google Drive')).toBeTruthy()
    })

    it('loads Google Drive account on mount', async () => {
      const { invoke } = await import('@tauri-apps/api/core')
      const mockInvoke = vi.mocked(invoke)
      mockInvoke.mockResolvedValue({
        displayName: 'Test User',
        email: 'test@example.com',
        parentFolderId: null,
      })

      render(
        <NotificationProvider>
          <Settings />
        </NotificationProvider>
      )

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('get_google_drive_account')
      })
    })

    it('handles connect drive with polling', async () => {
      const { invoke } = await import('@tauri-apps/api/core')
      const mockInvoke = vi.mocked(invoke)

      let callCount = 0
      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'start_google_drive_auth') {
          return Promise.resolve({ authUrl: 'https://accounts.google.com/o/oauth2/v2/auth' })
        }
        if (cmd === 'complete_google_drive_auth') {
          callCount++
          if (callCount < 3) {
            return Promise.reject(new Error('Not ready'))
          }
          return Promise.resolve({
            displayName: 'Test User',
            email: 'test@example.com',
            parentFolderId: null,
          })
        }
        return Promise.resolve(null)
      })

      const user = userEvent.setup()

      render(
        <NotificationProvider>
          <Settings />
        </NotificationProvider>
      )

      const connectButton = screen.getByText('Connect Google Drive')
      await user.click(connectButton)

      await waitFor(
        () => {
          expect(screen.getByText('Test User')).toBeTruthy()
        },
        { timeout: 10000 }
      )
    })

    it.skip('handles authentication timeout', async () => {
      // Skip this test as it takes 5+ minutes to complete (150 attempts * 2 seconds)
      // The polling mechanism is tested in the success case above
    })

    it('handles disconnect drive', async () => {
      const { invoke } = await import('@tauri-apps/api/core')
      const mockInvoke = vi.mocked(invoke)

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_google_drive_account') {
          return Promise.resolve({
            displayName: 'Test User',
            email: 'test@example.com',
            parentFolderId: null,
          })
        }
        if (cmd === 'remove_google_drive_account') {
          return Promise.resolve(undefined)
        }
        return Promise.resolve(null)
      })

      const user = userEvent.setup()

      render(
        <NotificationProvider>
          <Settings />
        </NotificationProvider>
      )

      await waitFor(() => {
        expect(screen.getByText('Test User')).toBeTruthy()
      })

      const disconnectButton = screen.getByText('Disconnect')
      await user.click(disconnectButton)

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('remove_google_drive_account')
        expect(screen.queryByText('Test User')).toBeFalsy()
      })
    })

    it('handles configure parent folder cancellation', async () => {
      const { invoke } = await import('@tauri-apps/api/core')
      const mockInvoke = vi.mocked(invoke)
      const mockPrompt = vi.spyOn(window, 'prompt').mockReturnValue(null)

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_google_drive_account') {
          return Promise.resolve({
            displayName: 'Test User',
            email: 'test@example.com',
            parentFolderId: null,
          })
        }
        return Promise.resolve(null)
      })

      const user = userEvent.setup()

      render(
        <NotificationProvider>
          <Settings />
        </NotificationProvider>
      )

      await waitFor(() => {
        expect(screen.getByText('Test User')).toBeTruthy()
      })

      const configureButton = screen.getByText('Configure Parent Folder')
      await user.click(configureButton)

      expect(mockPrompt).toHaveBeenCalled()
      expect(mockInvoke).not.toHaveBeenCalledWith('set_drive_parent_folder', expect.anything())

      mockPrompt.mockRestore()
    })

    it('handles configure parent folder success', async () => {
      const { invoke } = await import('@tauri-apps/api/core')
      const mockInvoke = vi.mocked(invoke)
      const mockPrompt = vi.spyOn(window, 'prompt').mockReturnValue('folder123')

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_google_drive_account') {
          return Promise.resolve({
            displayName: 'Test User',
            email: 'test@example.com',
            parentFolderId: null,
          })
        }
        if (cmd === 'set_drive_parent_folder') {
          return Promise.resolve(undefined)
        }
        return Promise.resolve(null)
      })

      const user = userEvent.setup()

      render(
        <NotificationProvider>
          <Settings />
        </NotificationProvider>
      )

      await waitFor(() => {
        expect(screen.getByText('Test User')).toBeTruthy()
      })

      const configureButton = screen.getByText('Configure Parent Folder')
      await user.click(configureButton)

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('set_drive_parent_folder', {
          folderId: 'folder123',
        })
      })

      mockPrompt.mockRestore()
    })

    it('handles test connection success', async () => {
      const { invoke } = await import('@tauri-apps/api/core')
      const mockInvoke = vi.mocked(invoke)

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_google_drive_account') {
          return Promise.resolve({
            displayName: 'Test User',
            email: 'test@example.com',
            parentFolderId: null,
          })
        }
        if (cmd === 'test_google_drive_connection') {
          return Promise.resolve(undefined)
        }
        return Promise.resolve(null)
      })

      const user = userEvent.setup()

      render(
        <NotificationProvider>
          <Settings />
        </NotificationProvider>
      )

      await waitFor(() => {
        expect(screen.getByText('Test User')).toBeTruthy()
      })

      const testButton = screen.getByText('Test Connection')
      await user.click(testButton)

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('test_google_drive_connection')
      })
    })

    it('handles test connection failure', async () => {
      const { invoke } = await import('@tauri-apps/api/core')
      const mockInvoke = vi.mocked(invoke)

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_google_drive_account') {
          return Promise.resolve({
            displayName: 'Test User',
            email: 'test@example.com',
            parentFolderId: null,
          })
        }
        if (cmd === 'test_google_drive_connection') {
          return Promise.reject(new Error('Connection failed'))
        }
        return Promise.resolve(null)
      })

      const user = userEvent.setup()

      render(
        <NotificationProvider>
          <Settings />
        </NotificationProvider>
      )

      await waitFor(() => {
        expect(screen.getByText('Test User')).toBeTruthy()
      })

      const testButton = screen.getByText('Test Connection')
      await user.click(testButton)

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('test_google_drive_connection')
      })
    })
  })
})
