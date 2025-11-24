import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import { userEvent } from '@testing-library/user-event'
import { CommandPalette } from './CommandPalette'

describe('CommandPalette', () => {
  it('renders nothing when not open', () => {
    const { container } = render(
      <CommandPalette isOpen={false} onClose={vi.fn()} onNavigate={vi.fn()} />
    )

    expect(container.firstChild).toBeNull()
  })

  it('renders when open', () => {
    render(<CommandPalette isOpen={true} onClose={vi.fn()} onNavigate={vi.fn()} />)

    expect(screen.getByPlaceholderText(/Search/)).toBeTruthy()
  })

  it('displays navigation commands', () => {
    render(<CommandPalette isOpen={true} onClose={vi.fn()} onNavigate={vi.fn()} />)

    expect(screen.getByText('Dashboard')).toBeTruthy()
    expect(screen.getByText('Import')).toBeTruthy()
    expect(screen.getByText('Projects')).toBeTruthy()
  })

  it('calls onClose when overlay clicked', async () => {
    const onClose = vi.fn()
    const user = userEvent.setup()

    render(<CommandPalette isOpen={true} onClose={onClose} onNavigate={vi.fn()} />)

    const overlay = document.querySelector('.command-palette-overlay')
    await user.click(overlay!)

    expect(onClose).toHaveBeenCalledTimes(1)
  })

  it('calls onNavigate when command selected', async () => {
    const onNavigate = vi.fn()
    const user = userEvent.setup()

    render(<CommandPalette isOpen={true} onClose={vi.fn()} onNavigate={onNavigate} />)

    const dashboardCommand = screen.getByText('Dashboard')
    await user.click(dashboardCommand)

    expect(onNavigate).toHaveBeenCalled()
  })
})
