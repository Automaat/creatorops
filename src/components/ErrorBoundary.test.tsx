import { describe, expect, it, vi } from 'vitest'
import { fireEvent, render, screen } from '@testing-library/react'

import { ErrorBoundary } from './ErrorBoundary'

function Thrower({ shouldThrow }: { shouldThrow: boolean }) {
  if (shouldThrow) {
    throw new Error('Test error message')
  }
  return <div>content</div>
}

function AlwaysThrows() {
  throw new Error('Component crashed')
}

describe('error boundary', () => {
  it('renders children when no error', () => {
    render(
      <ErrorBoundary>
        <div>content</div>
      </ErrorBoundary>
    )
    expect(screen.getByText('content')).toBeTruthy()
  })

  it('shows fallback UI on render error', () => {
    const spy = vi.spyOn(console, 'error').mockImplementation(() => {})
    render(
      <ErrorBoundary>
        <Thrower shouldThrow />
      </ErrorBoundary>
    )
    expect(screen.getByText('Something went wrong')).toBeTruthy()
    expect(screen.getByText('Test error message')).toBeTruthy()
    expect(screen.getByRole('button', { name: 'Try again' })).toBeTruthy()
    spy.mockRestore()
  })

  it('shows section name in fallback when name prop provided', () => {
    const spy = vi.spyOn(console, 'error').mockImplementation(() => {})
    render(
      <ErrorBoundary name="Dashboard">
        <Thrower shouldThrow />
      </ErrorBoundary>
    )
    expect(screen.getByText('in Dashboard')).toBeTruthy()
    spy.mockRestore()
  })

  it('resets to children after try again click', () => {
    const spy = vi.spyOn(console, 'error').mockImplementation(() => {})
    const { rerender } = render(
      <ErrorBoundary>
        <AlwaysThrows />
      </ErrorBoundary>
    )

    expect(screen.getByText('Something went wrong')).toBeTruthy()

    rerender(
      <ErrorBoundary>
        <div>recovered</div>
      </ErrorBoundary>
    )

    fireEvent.click(screen.getByRole('button', { name: 'Try again' }))

    expect(screen.getByText('recovered')).toBeTruthy()
    spy.mockRestore()
  })

  it('resets when isActive flips from inactive to active while in error', () => {
    const spy = vi.spyOn(console, 'error').mockImplementation(() => {})
    const { rerender } = render(
      <ErrorBoundary isActive={true}>
        <AlwaysThrows />
      </ErrorBoundary>
    )
    expect(screen.getByText('Something went wrong')).toBeTruthy()

    rerender(
      <ErrorBoundary isActive={false}>
        <div>recovered</div>
      </ErrorBoundary>
    )
    rerender(
      <ErrorBoundary isActive={true}>
        <div>recovered</div>
      </ErrorBoundary>
    )

    expect(screen.getByText('recovered')).toBeTruthy()
    spy.mockRestore()
  })

  it('does not reset while isActive stays false', () => {
    const spy = vi.spyOn(console, 'error').mockImplementation(() => {})
    const { rerender } = render(
      <ErrorBoundary isActive={true}>
        <AlwaysThrows />
      </ErrorBoundary>
    )
    expect(screen.getByText('Something went wrong')).toBeTruthy()

    rerender(
      <ErrorBoundary isActive={false}>
        <div>recovered</div>
      </ErrorBoundary>
    )

    expect(screen.getByText('Something went wrong')).toBeTruthy()
    spy.mockRestore()
  })

  it('logs error with component name to console', () => {
    const spy = vi.spyOn(console, 'error').mockImplementation(() => {})
    render(
      <ErrorBoundary name="Import">
        <Thrower shouldThrow />
      </ErrorBoundary>
    )
    expect(spy).toHaveBeenCalledWith(
      '[ErrorBoundary (Import)]',
      expect.any(Error),
      expect.anything()
    )
    spy.mockRestore()
  })
})
