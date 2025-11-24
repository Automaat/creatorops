import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import { userEvent } from '@testing-library/user-event'
import { KeyboardShortcutsHelp } from './KeyboardShortcutsHelp'

describe('KeyboardShortcutsHelp', () => {
  it('renders nothing when isOpen is false', () => {
    const { container } = render(<KeyboardShortcutsHelp isOpen={false} onClose={vi.fn()} />)
    expect(container.firstChild).toBeNull()
  })

  it('renders modal when isOpen is true', () => {
    render(<KeyboardShortcutsHelp isOpen={true} onClose={vi.fn()} />)
    expect(screen.getByText('Keyboard Shortcuts')).toBeTruthy()
  })

  it('displays navigation shortcuts', () => {
    render(<KeyboardShortcutsHelp isOpen={true} onClose={vi.fn()} />)

    expect(screen.getByText('Navigation')).toBeTruthy()
    expect(screen.getByText('Open Settings')).toBeTruthy()
    expect(screen.getByText('Show Keyboard Shortcuts')).toBeTruthy()
    expect(screen.getByText('Go to Dashboard')).toBeTruthy()
    expect(screen.getByText('Go to Import')).toBeTruthy()
    expect(screen.getByText('Go to Projects')).toBeTruthy()
    expect(screen.getByText('Go to Backup Queue')).toBeTruthy()
    expect(screen.getByText('Go to Delivery')).toBeTruthy()
    expect(screen.getByText('Go to History')).toBeTruthy()
    expect(screen.getByText('Refresh SD Cards')).toBeTruthy()
  })

  it('displays general shortcuts', () => {
    render(<KeyboardShortcutsHelp isOpen={true} onClose={vi.fn()} />)

    expect(screen.getByText('General')).toBeTruthy()
    expect(screen.getByText('Close dialogs')).toBeTruthy()
    expect(screen.getByText('Submit forms')).toBeTruthy()
  })

  it('calls onClose when close button clicked', async () => {
    const user = userEvent.setup()
    const onClose = vi.fn()

    render(<KeyboardShortcutsHelp isOpen={true} onClose={onClose} />)

    const closeButton = screen.getByLabelText('Close')
    await user.click(closeButton)

    expect(onClose).toHaveBeenCalledTimes(1)
  })

  it('calls onClose when overlay clicked', async () => {
    const user = userEvent.setup()
    const onClose = vi.fn()

    render(<KeyboardShortcutsHelp isOpen={true} onClose={onClose} />)

    const overlay = document.querySelector('.shortcuts-overlay')
    await user.click(overlay!)

    expect(onClose).toHaveBeenCalledTimes(1)
  })

  it('does not close when modal content clicked', async () => {
    const user = userEvent.setup()
    const onClose = vi.fn()

    render(<KeyboardShortcutsHelp isOpen={true} onClose={onClose} />)

    const modal = document.querySelector('.shortcuts-modal')
    await user.click(modal!)

    expect(onClose).not.toHaveBeenCalled()
  })

  it('displays keyboard shortcut keys correctly', () => {
    render(<KeyboardShortcutsHelp isOpen={true} onClose={vi.fn()} />)

    // Check for meta key shortcuts (⌘)
    const metaKeys = screen.getAllByText('⌘')
    expect(metaKeys.length).toBeGreaterThan(0)

    // Check for specific key combinations
    expect(screen.getByText(',')).toBeTruthy() // Settings
    expect(screen.getByText('/')).toBeTruthy() // Show shortcuts
    expect(screen.getAllByText('1').length).toBeGreaterThan(0) // Dashboard
    expect(screen.getAllByText('R').length).toBeGreaterThan(0) // Refresh
  })
})
