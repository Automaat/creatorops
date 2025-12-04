import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { act, renderHook, waitFor } from '@testing-library/react'
import { useSDCardScanner } from './useSDCardScanner'
import { NotificationProvider } from '../contexts/NotificationContext'
import { invoke } from '@tauri-apps/api/core'
import type { SDCard } from '../types'

// Mock Tauri API
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}))

vi.mock('@tauri-apps/plugin-notification', () => ({
  isPermissionGranted: vi.fn(), requestPermission: vi.fn(), sendNotification: vi.fn(),
}))

const mockInvoke = vi.mocked(invoke)

const createMockCard = (id: number, size: number = 1000): SDCard => ({
  deviceType: 'SD', fileCount: id * 10, freeSpace: size / 2, isRemovable: true, name: `Card ${id}`, path: `/path/${id}`, size,
})

describe('useSDCardScanner', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  it('initializes with empty sdCards array', async () => {
    mockInvoke.mockResolvedValue([])

    const { result, unmount } = renderHook(() => useSDCardScanner(), {
      wrapper: NotificationProvider,
    })

    expect(result.current.sdCards).toStrictEqual([])

    // Wait for initial scan to complete
    await waitFor(() => {
      expect(result.current.isScanning).toBe(false)
    })

    unmount()
  })

  it('scans for SD cards on mount', async () => {
    const mockCards: SDCard[] = [createMockCard(1)]

    mockInvoke.mockResolvedValue(mockCards)

    const { result, unmount } = renderHook(() => useSDCardScanner(), {
      wrapper: NotificationProvider,
    })

    await waitFor(() => {
      expect(result.current.sdCards).toStrictEqual(mockCards)
    })

    unmount()
  })

  it('sets isScanning state during scan', async () => {
    let resolveInvoke: (value: SDCard[]) => void
    mockInvoke.mockReturnValue(
      new Promise((resolve) => {
        resolveInvoke = resolve
      })
    )

    const { result, unmount } = renderHook(() => useSDCardScanner(), {
      wrapper: NotificationProvider,
    })

    await waitFor(() => {
      expect(result.current.isScanning).toBe(true)
    })

    await act(async () => {
      resolveInvoke!([])
      await Promise.resolve()
    })

    await waitFor(() => {
      expect(result.current.isScanning).toBe(false)
    })

    unmount()
  })

  it('triggers onCardDetected callback for new cards', async () => {
    const onCardDetected = vi.fn()
    const mockCards: SDCard[] = [createMockCard(1)]

    mockInvoke.mockResolvedValueOnce([])
    mockInvoke.mockResolvedValueOnce(mockCards)

    const { result } = renderHook(() => useSDCardScanner({ onCardDetected }), {
      wrapper: NotificationProvider,
    })

    await waitFor(() => {
      expect(result.current.sdCards).toStrictEqual([])
    })

    await act(async () => {
      await result.current.scanForSDCards()
    })

    await waitFor(() => {
      expect(onCardDetected).toHaveBeenCalled()
    })
  })

  it('auto-scans at regular intervals', async () => {
    vi.useFakeTimers()
    mockInvoke.mockResolvedValue([])

    const { unmount } = renderHook(() => useSDCardScanner(), {
      wrapper: NotificationProvider,
    })

    // Wait for initial scan (flush promises without advancing timers)
    await act(async () => {
      await Promise.resolve()
    })

    expect(invoke).toHaveBeenCalledTimes(1)

    // First interval scan
    await act(async () => {
      await vi.advanceTimersByTimeAsync(5000)
    })

    expect(invoke).toHaveBeenCalledTimes(2)

    // Second interval scan
    await act(async () => {
      await vi.advanceTimersByTimeAsync(5000)
    })

    expect(invoke).toHaveBeenCalledTimes(3)

    unmount()
    vi.useRealTimers()
  })

  it('handles scan errors gracefully', async () => {
    vi.useFakeTimers()
    const consoleError = console.error
    const spy = vi.spyOn(console, 'error').mockImplementation(() => {})

    mockInvoke.mockRejectedValue(new Error('Scan failed'))

    const { result, unmount } = renderHook(() => useSDCardScanner(), {
      wrapper: NotificationProvider,
    })

    await act(async () => {
      await Promise.resolve()
    })

    expect(result.current.isScanning).toBe(false)
    expect(console.error).toHaveBeenCalledWith('Failed to scan SD cards:', expect.any(Error))

    unmount()
    spy.mockRestore()
    console.error = consoleError
    vi.useRealTimers()
  })

  it('provides manual scanForSDCards function', async () => {
    vi.useFakeTimers()
    const mockCards: SDCard[] = [createMockCard(1)]

    mockInvoke.mockResolvedValue(mockCards)

    const { result, unmount } = renderHook(() => useSDCardScanner(), {
      wrapper: NotificationProvider,
    })

    await act(async () => {
      await Promise.resolve()
    })

    expect(result.current.sdCards).toStrictEqual(mockCards)

    const mockCards2: SDCard[] = [createMockCard(2, 2000)]

    mockInvoke.mockResolvedValue(mockCards2)

    await act(async () => {
      await result.current.scanForSDCards()
    })

    expect(result.current.sdCards).toStrictEqual(mockCards2)

    unmount()
    vi.useRealTimers()
  })

  it('cleans up interval on unmount', async () => {
    vi.useFakeTimers()
    mockInvoke.mockResolvedValue([])

    const { unmount } = renderHook(() => useSDCardScanner(), {
      wrapper: NotificationProvider,
    })

    // Wait for initial scan (flush promises without advancing timers)
    await act(async () => {
      await Promise.resolve()
    })

    expect(invoke).toHaveBeenCalledTimes(1)

    // Unmount the hook - this should clear the interval
    await act(async () => {
      unmount()
    })

    // Advance timers - should not trigger any more scans
    await act(async () => {
      await vi.advanceTimersByTimeAsync(10_000)
    })

    // Should not have been called again after unmount
    expect(invoke).toHaveBeenCalledTimes(1)

    vi.useRealTimers()
  })
})
