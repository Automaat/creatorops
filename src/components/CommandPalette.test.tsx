import { beforeEach, describe, expect, it, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import { userEvent } from '@testing-library/user-event'
import { CommandPalette } from './CommandPalette'
import { invoke } from '@tauri-apps/api/core'

// Mock Tauri API
vi.mock<typeof import('@tauri-apps/api/core')>('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}))

const mockInvoke = vi.mocked(invoke)

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
})
