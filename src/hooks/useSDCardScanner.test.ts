import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { renderHook, waitFor, act } from '@testing-library/react'
import { useSDCardScanner } from './useSDCardScanner'
import { NotificationProvider } from '../contexts/NotificationContext'
import { invoke } from '@tauri-apps/api/core'
import type { SDCard } from '../types'

// Mock Tauri API
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}))

vi.mock('@tauri-apps/plugin-notification', () => ({
  sendNotification: vi.fn(),
  isPermissionGranted: vi.fn(),
  requestPermission: vi.fn(),
}))

const mockInvoke = vi.mocked(invoke)

describe('useSDCardScanner', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    vi.useFakeTimers()
  })

  afterEach(() => {
    vi.restoreAllMocks()
    vi.useRealTimers()
  })

  it('initializes with empty sdCards array', () => {
    
    mockInvoke.mockResolvedValue([])

    const { result } = renderHook(() => useSDCardScanner(), {
      wrapper: NotificationProvider,
    })

    expect(result.current.sdCards).toEqual([])
    expect(result.current.isScanning).toBe(false)
  })

  it('scans for SD cards on mount', async () => {
    const mockCards: SDCard[] = [
      { name: 'Card 1', path: '/path/1', size: 1000, free_space: 500 },
    ]

    
    mockInvoke.mockResolvedValue(mockCards)

    const { result } = renderHook(() => useSDCardScanner(), {
      wrapper: NotificationProvider,
    })

    await waitFor(() => {
      expect(result.current.sdCards).toEqual(mockCards)
    })
  })

  it('sets isScanning state during scan', async () => {
    
    let resolveInvoke: (value: SDCard[]) => void
    invoke.mockReturnValue(
      new Promise((resolve) => {
        resolveInvoke = resolve
      })
    )

    const { result } = renderHook(() => useSDCardScanner(), {
      wrapper: NotificationProvider,
    })

    await waitFor(() => {
      expect(result.current.isScanning).toBe(true)
    })

    act(() => {
      resolveInvoke!([])
    })

    await waitFor(() => {
      expect(result.current.isScanning).toBe(false)
    })
  })

  it('triggers onCardDetected callback for new cards', async () => {
    const onCardDetected = vi.fn()
    const mockCards: SDCard[] = [
      { name: 'Card 1', path: '/path/1', size: 1000, free_space: 500 },
    ]

    
    mockInvoke.mockResolvedValueOnce([])
    mockInvoke.mockResolvedValueOnce(mockCards)

    const { result } = renderHook(() => useSDCardScanner({ onCardDetected }), {
      wrapper: NotificationProvider,
    })

    await waitFor(() => {
      expect(result.current.sdCards).toEqual([])
    })

    act(() => {
      result.current.scanForSDCards()
    })

    await waitFor(() => {
      expect(onCardDetected).toHaveBeenCalled()
    })
  })

  it('auto-scans at regular intervals', async () => {
    
    mockInvoke.mockResolvedValue([])

    renderHook(() => useSDCardScanner(), {
      wrapper: NotificationProvider,
    })

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledTimes(1)
    })

    act(() => {
      vi.advanceTimersByTime(5000)
    })

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledTimes(2)
    })

    act(() => {
      vi.advanceTimersByTime(5000)
    })

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledTimes(3)
    })
  })

  it('handles scan errors gracefully', async () => {
    const consoleError = console.error
    console.error = vi.fn()

    
    mockInvoke.mockRejectedValue(new Error('Scan failed'))

    const { result } = renderHook(() => useSDCardScanner(), {
      wrapper: NotificationProvider,
    })

    await waitFor(() => {
      expect(result.current.isScanning).toBe(false)
    })

    expect(console.error).toHaveBeenCalledWith('Failed to scan SD cards:', expect.any(Error))

    console.error = consoleError
  })

  it('provides manual scanForSDCards function', async () => {
    const mockCards: SDCard[] = [
      { name: 'Card 1', path: '/path/1', size: 1000, free_space: 500 },
    ]

    
    mockInvoke.mockResolvedValue(mockCards)

    const { result } = renderHook(() => useSDCardScanner(), {
      wrapper: NotificationProvider,
    })

    await waitFor(() => {
      expect(result.current.sdCards).toEqual(mockCards)
    })

    const mockCards2: SDCard[] = [
      { name: 'Card 2', path: '/path/2', size: 2000, free_space: 1000 },
    ]

    mockInvoke.mockResolvedValue(mockCards2)

    act(() => {
      result.current.scanForSDCards()
    })

    await waitFor(() => {
      expect(result.current.sdCards).toEqual(mockCards2)
    })
  })

  it('cleans up interval on unmount', async () => {
    
    mockInvoke.mockResolvedValue([])

    const { unmount } = renderHook(() => useSDCardScanner(), {
      wrapper: NotificationProvider,
    })

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledTimes(1)
    })

    unmount()

    act(() => {
      vi.advanceTimersByTime(10000)
    })

    // Should not have been called again after unmount
    expect(invoke).toHaveBeenCalledTimes(1)
  })
})
