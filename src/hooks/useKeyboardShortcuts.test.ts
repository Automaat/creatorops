import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { renderHook } from '@testing-library/react'
import { useKeyboardShortcuts, type KeyboardShortcut } from './useKeyboardShortcuts'

describe('useKeyboardShortcuts', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  it('triggers action on matching key', () => {
    const action = vi.fn()
    const shortcuts: KeyboardShortcut[] = [
      { key: 'a', description: 'Test', action },
    ]

    renderHook(() => useKeyboardShortcuts(shortcuts))

    const event = new KeyboardEvent('keydown', { key: 'a' })
    window.dispatchEvent(event)

    expect(action).toHaveBeenCalledTimes(1)
  })

  it('triggers action with meta key', () => {
    const action = vi.fn()
    const shortcuts: KeyboardShortcut[] = [
      { key: 'k', metaKey: true, description: 'Test', action },
    ]

    renderHook(() => useKeyboardShortcuts(shortcuts))

    const event = new KeyboardEvent('keydown', { key: 'k', metaKey: true })
    window.dispatchEvent(event)

    expect(action).toHaveBeenCalledTimes(1)
  })

  it('triggers action with ctrl key', () => {
    const action = vi.fn()
    const shortcuts: KeyboardShortcut[] = [
      { key: 'c', ctrlKey: true, description: 'Test', action },
    ]

    renderHook(() => useKeyboardShortcuts(shortcuts))

    const event = new KeyboardEvent('keydown', { key: 'c', ctrlKey: true })
    window.dispatchEvent(event)

    expect(action).toHaveBeenCalledTimes(1)
  })

  it('triggers action with shift key', () => {
    const action = vi.fn()
    const shortcuts: KeyboardShortcut[] = [
      { key: 's', shiftKey: true, description: 'Test', action },
    ]

    renderHook(() => useKeyboardShortcuts(shortcuts))

    const event = new KeyboardEvent('keydown', { key: 's', shiftKey: true })
    window.dispatchEvent(event)

    expect(action).toHaveBeenCalledTimes(1)
  })

  it('triggers action with alt key', () => {
    const action = vi.fn()
    const shortcuts: KeyboardShortcut[] = [
      { key: 'a', altKey: true, description: 'Test', action },
    ]

    renderHook(() => useKeyboardShortcuts(shortcuts))

    const event = new KeyboardEvent('keydown', { key: 'a', altKey: true })
    window.dispatchEvent(event)

    expect(action).toHaveBeenCalledTimes(1)
  })

  it('does not trigger without matching modifier keys', () => {
    const action = vi.fn()
    const shortcuts: KeyboardShortcut[] = [
      { key: 'k', metaKey: true, description: 'Test', action },
    ]

    renderHook(() => useKeyboardShortcuts(shortcuts))

    const event = new KeyboardEvent('keydown', { key: 'k' })
    window.dispatchEvent(event)

    expect(action).not.toHaveBeenCalled()
  })

  it('handles multiple shortcuts', () => {
    const action1 = vi.fn()
    const action2 = vi.fn()
    const shortcuts: KeyboardShortcut[] = [
      { key: 'a', description: 'Test 1', action: action1 },
      { key: 'b', description: 'Test 2', action: action2 },
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
    const shortcuts: KeyboardShortcut[] = [
      { key: 'A', description: 'Test', action },
    ]

    renderHook(() => useKeyboardShortcuts(shortcuts))

    const event = new KeyboardEvent('keydown', { key: 'a' })
    window.dispatchEvent(event)

    expect(action).toHaveBeenCalledTimes(1)
  })

  it('prevents default when shortcut matches', () => {
    const action = vi.fn()
    const shortcuts: KeyboardShortcut[] = [
      { key: 'k', metaKey: true, description: 'Test', action },
    ]

    renderHook(() => useKeyboardShortcuts(shortcuts))

    const event = new KeyboardEvent('keydown', { key: 'k', metaKey: true })
    const preventDefaultSpy = vi.spyOn(event, 'preventDefault')

    window.dispatchEvent(event)

    expect(preventDefaultSpy).toHaveBeenCalled()
  })

  it('does not trigger when disabled', () => {
    const action = vi.fn()
    const shortcuts: KeyboardShortcut[] = [
      { key: 'a', description: 'Test', action },
    ]

    renderHook(() => useKeyboardShortcuts(shortcuts, false))

    const event = new KeyboardEvent('keydown', { key: 'a' })
    window.dispatchEvent(event)

    expect(action).not.toHaveBeenCalled()
  })

  it('cleans up event listeners on unmount', () => {
    const action = vi.fn()
    const shortcuts: KeyboardShortcut[] = [
      { key: 'a', description: 'Test', action },
    ]

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
      { key: 'a', description: 'Test 1', action: action1 },
      { key: 'a', description: 'Test 2', action: action2 },
    ]

    renderHook(() => useKeyboardShortcuts(shortcuts))

    const event = new KeyboardEvent('keydown', { key: 'a' })
    window.dispatchEvent(event)

    expect(action1).toHaveBeenCalledTimes(1)
    expect(action2).not.toHaveBeenCalled()
  })
})
