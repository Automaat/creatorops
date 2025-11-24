import { describe, it, expect, beforeEach, afterEach } from 'vitest'
import { renderHook, act } from '@testing-library/react'
import { useTheme } from './useTheme'

describe('useTheme', () => {
  beforeEach(() => {
    localStorage.clear()
    document.documentElement.removeAttribute('data-theme')
  })

  afterEach(() => {
    localStorage.clear()
    document.documentElement.removeAttribute('data-theme')
  })

  it('initializes with system theme by default', () => {
    const { result } = renderHook(() => useTheme())
    expect(result.current.theme).toBe('system')
  })

  it('loads theme from localStorage', () => {
    localStorage.setItem('theme', 'dark')
    const { result } = renderHook(() => useTheme())
    expect(result.current.theme).toBe('dark')
  })

  it('sets light theme', () => {
    const { result } = renderHook(() => useTheme())

    act(() => {
      result.current.setTheme('light')
    })

    expect(result.current.theme).toBe('light')
    expect(document.documentElement.getAttribute('data-theme')).toBe('light')
    expect(localStorage.getItem('theme')).toBe('light')
  })

  it('sets dark theme', () => {
    const { result } = renderHook(() => useTheme())

    act(() => {
      result.current.setTheme('dark')
    })

    expect(result.current.theme).toBe('dark')
    expect(document.documentElement.getAttribute('data-theme')).toBe('dark')
    expect(localStorage.getItem('theme')).toBe('dark')
  })

  it('removes data-theme attribute for system theme', () => {
    const { result } = renderHook(() => useTheme())

    act(() => {
      result.current.setTheme('dark')
    })

    expect(document.documentElement.getAttribute('data-theme')).toBe('dark')

    act(() => {
      result.current.setTheme('system')
    })

    expect(result.current.theme).toBe('system')
    expect(document.documentElement.getAttribute('data-theme')).toBeNull()
    expect(localStorage.getItem('theme')).toBe('system')
  })

  it('persists theme changes to localStorage', () => {
    const { result } = renderHook(() => useTheme())

    act(() => {
      result.current.setTheme('light')
    })

    expect(localStorage.getItem('theme')).toBe('light')

    act(() => {
      result.current.setTheme('dark')
    })

    expect(localStorage.getItem('theme')).toBe('dark')
  })
})
