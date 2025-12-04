import { beforeEach, describe, expect, it, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import { userEvent } from '@testing-library/user-event'
import { DatePicker } from './DatePicker'

describe('datePicker', () => {
  const mockOnChange = vi.fn()

  beforeEach(() => {
    mockOnChange.mockClear()
  })

  it('renders with placeholder when no value', () => {
    render(<DatePicker value="" onChange={mockOnChange} />)
    expect(screen.getByText('Select date')).toBeTruthy()
  })

  it('renders with formatted date when value provided', () => {
    render(<DatePicker value="2025-11-20" onChange={mockOnChange} />)
    expect(screen.getByText('Nov 20, 2025')).toBeTruthy()
  })

  it('renders with label when provided', () => {
    render(<DatePicker value="" onChange={mockOnChange} label="Pick a date" />)
    expect(screen.getByText('Pick a date')).toBeTruthy()
  })

  it('shows required indicator when required prop is true', () => {
    render(<DatePicker value="" onChange={mockOnChange} label="Date" required />)
    const label = screen.getByText(/Date/)
    expect(label.textContent).toContain('*')
  })

  it('opens calendar when input clicked', async () => {
    const user = userEvent.setup()
    render(<DatePicker value="" onChange={mockOnChange} />)

    const input = screen.getByText('Select date')
    await user.click(input)

    // Calendar should be visible
    expect(document.querySelector('.date-picker-dropdown')).toBeTruthy()
  })

  it('closes calendar when clicking outside', async () => {
    const user = userEvent.setup()
    render(
      <div>
        <DatePicker value="2025-11-20" onChange={mockOnChange} />
        <div data-testid="outside">Outside</div>
      </div>
    )

    const input = screen.getByText('Nov 20, 2025')
    await user.click(input)

    expect(document.querySelector('.date-picker-dropdown')).toBeTruthy()

    const outside = screen.getByTestId('outside')
    await user.click(outside)

    expect(document.querySelector('.date-picker-dropdown')).toBeNull()
  })

  it('displays correct month and year in header', async () => {
    const user = userEvent.setup()
    render(<DatePicker value="2025-11-20" onChange={mockOnChange} />)

    const input = screen.getByText('Nov 20, 2025')
    await user.click(input)

    expect(screen.getByText('November 2025')).toBeTruthy()
  })

  it('navigates to previous month', async () => {
    const user = userEvent.setup()
    render(<DatePicker value="2025-11-20" onChange={mockOnChange} />)

    const input = screen.getByText('Nov 20, 2025')
    await user.click(input)

    const prevButton = screen.getByText('←')
    await user.click(prevButton)

    expect(screen.getByText('October 2025')).toBeTruthy()
  })

  it('navigates to next month', async () => {
    const user = userEvent.setup()
    render(<DatePicker value="2025-11-20" onChange={mockOnChange} />)

    const input = screen.getByText('Nov 20, 2025')
    await user.click(input)

    const nextButton = screen.getByText('→')
    await user.click(nextButton)

    expect(screen.getByText('December 2025')).toBeTruthy()
  })

  it('displays weekday headers', async () => {
    const user = userEvent.setup()
    render(<DatePicker value="" onChange={mockOnChange} />)

    const input = screen.getByText('Select date')
    await user.click(input)

    expect(screen.getByText('Su')).toBeTruthy()
    expect(screen.getByText('Mo')).toBeTruthy()
    expect(screen.getByText('Tu')).toBeTruthy()
    expect(screen.getByText('We')).toBeTruthy()
    expect(screen.getByText('Th')).toBeTruthy()
    expect(screen.getByText('Fr')).toBeTruthy()
    expect(screen.getByText('Sa')).toBeTruthy()
  })

  it('calls onChange with formatted date when day selected', async () => {
    const user = userEvent.setup()
    render(<DatePicker value="" onChange={mockOnChange} />)

    const input = screen.getByText('Select date')
    await user.click(input)

    // Select day 15
    const days = screen.getAllByText('15')
    await user.click(days[0])

    expect(mockOnChange).toHaveBeenCalled()
    const firstCall = mockOnChange.mock.calls[0]
    expect(firstCall).toBeDefined()
    const callArg: unknown = firstCall?.[0]
    expect(typeof callArg).toBe('string')
    if (typeof callArg === 'string') {
      expect(callArg).toMatch(/^\d{4}-\d{2}-15$/)
    }
  })

  it('closes calendar after selecting date', async () => {
    const user = userEvent.setup()
    render(<DatePicker value="" onChange={mockOnChange} />)

    const input = screen.getByText('Select date')
    await user.click(input)

    expect(document.querySelector('.date-picker-dropdown')).toBeTruthy()

    const days = screen.getAllByText('15')
    await user.click(days[0])

    expect(document.querySelector('.date-picker-dropdown')).toBeNull()
  })

  it('highlights selected date', async () => {
    const user = userEvent.setup()
    render(<DatePicker value="2025-11-20" onChange={mockOnChange} />)

    const input = screen.getByText('Nov 20, 2025')
    await user.click(input)

    const days = [...document.querySelectorAll('.date-picker-day')]
    const selectedDay = days.find((day) => day.classList.contains('selected'))

    expect(selectedDay).toBeTruthy()
    expect(selectedDay?.textContent).toBe('20')
  })

  it("highlights today's date", async () => {
    const user = userEvent.setup()
    const today = new Date()
    const year = today.getFullYear()
    const month = String(today.getMonth() + 1).padStart(2, '0')
    const day = String(today.getDate()).padStart(2, '0')

    render(<DatePicker value={`${year}-${month}-${day}`} onChange={mockOnChange} />)

    const input = screen.getByRole('button')
    await user.click(input)

    const days = [...document.querySelectorAll('.date-picker-day')]
    const todayDay = days.find((d) => d.classList.contains('today'))

    expect(todayDay).toBeTruthy()
  })

  it('auto-opens calendar when autoOpen is true', () => {
    render(<DatePicker value="" onChange={mockOnChange} autoOpen />)

    expect(document.querySelector('.date-picker-dropdown')).toBeTruthy()
  })

  it('updates when value prop changes', () => {
    const { rerender } = render(<DatePicker value="2025-11-20" onChange={mockOnChange} />)

    expect(screen.getByText('Nov 20, 2025')).toBeTruthy()

    rerender(<DatePicker value="2025-12-25" onChange={mockOnChange} />)

    expect(screen.getByText('Dec 25, 2025')).toBeTruthy()
  })

  it('handles empty value correctly', () => {
    const { rerender } = render(<DatePicker value="2025-11-20" onChange={mockOnChange} />)

    expect(screen.getByText('Nov 20, 2025')).toBeTruthy()

    rerender(<DatePicker value="" onChange={mockOnChange} />)

    expect(screen.getByText('Select date')).toBeTruthy()
  })

  it('formats output date as YYYY-MM-DD', async () => {
    const user = userEvent.setup()
    render(<DatePicker value="" onChange={mockOnChange} />)

    const input = screen.getByText('Select date')
    await user.click(input)

    const days = screen.getAllByText('5')
    await user.click(days[0])

    const firstCall = mockOnChange.mock.calls[0]
    expect(firstCall).toBeDefined()
    const callArg: unknown = firstCall?.[0]
    expect(typeof callArg).toBe('string')
    if (typeof callArg === 'string') {
      expect(callArg).toMatch(/^\d{4}-\d{2}-\d{2}$/)
    }
  })
})
