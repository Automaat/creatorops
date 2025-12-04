import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { renderHook } from '@testing-library/react'
import { useKeyboardShortcuts } from './useKeyboardShortcuts';
import type { KeyboardShortcut } from './useKeyboardShortcuts';

describe('useKeyboardShortcuts', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  it('triggers action on matching key', () => {
    const action = vi.fn()
    const shortcuts: KeyboardShortcut[] = [{ action, description: 'Test', key: 'a' }]

    renderHook(() => useKeyboardShortcuts(shortcuts))

    const event = new KeyboardEvent('keydown', { key: 'a' })
    window.dispatchEvent(event)

    expect(action).toHaveBeenCalledTimes(1)
  })

  it('triggers action with meta key', () => {
    const action = vi.fn()
    const shortcuts: KeyboardShortcut[] = [{ action, description: 'Test', key: 'k', metaKey: true }]

    renderHook(() => useKeyboardShortcuts(shortcuts))

    const event = new KeyboardEvent('keydown', { key: 'k', metaKey: true })
    window.dispatchEvent(event)

    expect(action).toHaveBeenCalledTimes(1)
  })

  it('triggers action with ctrl key', () => {
    const action = vi.fn()
    const shortcuts: KeyboardShortcut[] = [{ action, ctrlKey: true, description: 'Test', key: 'c' }]

    renderHook(() => useKeyboardShortcuts(shortcuts))

    const event = new KeyboardEvent('keydown', { ctrlKey: true, key: 'c' })
    window.dispatchEvent(event)

    expect(action).toHaveBeenCalledTimes(1)
  })

  it('triggers action with shift key', () => {
    const action = vi.fn()
    const shortcuts: KeyboardShortcut[] = [
      { action, description: 'Test', key: 's', shiftKey: true },
    ]

    renderHook(() => useKeyboardShortcuts(shortcuts))

    const event = new KeyboardEvent('keydown', { key: 's', shiftKey: true })
    window.dispatchEvent(event)

    expect(action).toHaveBeenCalledTimes(1)
  })

  it('triggers action with alt key', () => {
    const action = vi.fn()
    const shortcuts: KeyboardShortcut[] = [{ action, altKey: true, description: 'Test', key: 'a' }]

    renderHook(() => useKeyboardShortcuts(shortcuts))

    const event = new KeyboardEvent('keydown', { altKey: true, key: 'a' })
    window.dispatchEvent(event)

    expect(action).toHaveBeenCalledTimes(1)
  })

  it('does not trigger without matching modifier keys', () => {
    const action = vi.fn()
    const shortcuts: KeyboardShortcut[] = [{ action, description: 'Test', key: 'k', metaKey: true }]

    renderHook(() => useKeyboardShortcuts(shortcuts))

    const event = new KeyboardEvent('keydown', { key: 'k' })
    window.dispatchEvent(event)

    expect(action).not.toHaveBeenCalled()
  })

  it('handles multiple shortcuts', () => {
    const action1 = vi.fn()
    const action2 = vi.fn()
    const shortcuts: KeyboardShortcut[] = [
      { action: action1, description: 'Test 1', key: 'a' },
      { action: action2, description: 'Test 2', key: 'b' },
    ]

    renderHook(() => useKeyboardShortcuts(shortcuts))

    const event1 = new KeyboardEvent('keydown', { key: 'a' })
    window.dispatchEvent(event1)

    expect(action1).toHaveBeenCalledTimes(1)
    expect(action2).not.toHaveBeenCalled()

    const event2 = new KeyboardEvent('keydown', { key: 'b' })
    window.dispatchEvent(event2)

    expect(action2).toHaveBeenCalledTimes(1)
  })

  it('is case insensitive', () => {
    const action = vi.fn()
    const shortcuts: KeyboardShortcut[] = [{ action, description: 'Test', key: 'A' }]

    renderHook(() => useKeyboardShortcuts(shortcuts))

    const event = new KeyboardEvent('keydown', { key: 'a' })
    window.dispatchEvent(event)

    expect(action).toHaveBeenCalledTimes(1)
  })

  it('prevents default when shortcut matches', () => {
    const action = vi.fn()
    const shortcuts: KeyboardShortcut[] = [{ action, description: 'Test', key: 'k', metaKey: true }]

    renderHook(() => useKeyboardShortcuts(shortcuts))

    const event = new KeyboardEvent('keydown', { key: 'k', metaKey: true })
    const preventDefaultSpy = vi.spyOn(event, 'preventDefault')

    window.dispatchEvent(event)

    expect(preventDefaultSpy).toHaveBeenCalled()
  })

  it('does not trigger when disabled', () => {
    const action = vi.fn()
    const shortcuts: KeyboardShortcut[] = [{ action, description: 'Test', key: 'a' }]

    renderHook(() => useKeyboardShortcuts(shortcuts, false))

    const event = new KeyboardEvent('keydown', { key: 'a' })
    window.dispatchEvent(event)

    expect(action).not.toHaveBeenCalled()
  })

  it('cleans up event listeners on unmount', () => {
    const action = vi.fn()
    const shortcuts: KeyboardShortcut[] = [{ action, description: 'Test', key: 'a' }]

    const { unmount } = renderHook(() => useKeyboardShortcuts(shortcuts))

    unmount()

    const event = new KeyboardEvent('keydown', { key: 'a' })
    window.dispatchEvent(event)

    expect(action).not.toHaveBeenCalled()
  })

  it('only triggers first matching shortcut', () => {
    const action1 = vi.fn()
    const action2 = vi.fn()
    const shortcuts: KeyboardShortcut[] = [
      { action: action1, description: 'Test 1', key: 'a' },
      { action: action2, description: 'Test 2', key: 'a' },
    ]

    renderHook(() => useKeyboardShortcuts(shortcuts))

    const event = new KeyboardEvent('keydown', { key: 'a' })
    window.dispatchEvent(event)

    expect(action1).toHaveBeenCalledTimes(1)
    expect(action2).not.toHaveBeenCalled()
  })
})
