import { describe, expect, it } from 'vitest'
import { renderHook } from '@testing-library/react'
import { useNotification } from './useNotification'
import { NotificationProvider } from '../contexts/NotificationContext'

describe('useNotification', () => {
  it('returns notification context when inside provider', () => {
    const { result } = renderHook(() => useNotification(), {
      wrapper: NotificationProvider,
    })

    expect(result.current).toBeDefined()
    expect(result.current.notifications).toBeDefined()
    expect(typeof result.current.addNotification).toBe('function')
    expect(typeof result.current.removeNotification).toBe('function')
    expect(typeof result.current.success).toBe('function')
    expect(typeof result.current.error).toBe('function')
    expect(typeof result.current.warning).toBe('function')
    expect(typeof result.current.info).toBe('function')
  })

  it('throws error when used outside provider', () => {
    // Suppress console.error for this test
    const consoleError = console.error
    console.error = () => {}

    expect(() => {
      renderHook(() => useNotification())
    }).toThrow('useNotification must be used within NotificationProvider')

    console.error = consoleError
  })
})
