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

const createMockCard = (id: number, size: number = 1000): SDCard => ({
  name: `Card ${id}`,
  path: `/path/${id}`,
  size,
  freeSpace: size / 2,
  fileCount: id * 10,
  deviceType: 'SD',
  isRemovable: true,
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

    const { result } = renderHook(() => useSDCardScanner(), {
      wrapper: NotificationProvider,
    })

    expect(result.current.sdCards).toEqual([])

    // Wait for initial scan to complete
    await waitFor(() => {
      expect(result.current.isScanning).toBe(false)
    })
  })

  it('scans for SD cards on mount', async () => {
    const mockCards: SDCard[] = [createMockCard(1)]

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
    mockInvoke.mockReturnValue(
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

    await act(async () => {
      resolveInvoke!([])
      await Promise.resolve()
    })

    await waitFor(() => {
      expect(result.current.isScanning).toBe(false)
    })
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
      expect(result.current.sdCards).toEqual([])
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

    await act(async () => {
      renderHook(() => useSDCardScanner(), {
        wrapper: NotificationProvider,
      })
    })

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledTimes(1)
    })

    await act(async () => {
      await vi.advanceTimersByTimeAsync(5000)
    })

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledTimes(2)
    })

    await act(async () => {
      await vi.advanceTimersByTimeAsync(5000)
    })

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledTimes(3)
    })

    vi.useRealTimers()
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
    const mockCards: SDCard[] = [createMockCard(1)]

    mockInvoke.mockResolvedValue(mockCards)

    const { result } = renderHook(() => useSDCardScanner(), {
      wrapper: NotificationProvider,
    })

    await waitFor(() => {
      expect(result.current.sdCards).toEqual(mockCards)
    })

    const mockCards2: SDCard[] = [createMockCard(2, 2000)]

    mockInvoke.mockResolvedValue(mockCards2)

    await act(async () => {
      await result.current.scanForSDCards()
    })

    await waitFor(() => {
      expect(result.current.sdCards).toEqual(mockCards2)
    })
  })

  it('cleans up interval on unmount', async () => {
    vi.useFakeTimers()
    mockInvoke.mockResolvedValue([])

    let unmount: () => void

    await act(async () => {
      const result = renderHook(() => useSDCardScanner(), {
        wrapper: NotificationProvider,
      })
      unmount = result.unmount
    })

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledTimes(1)
    })

    await act(async () => {
      unmount!()
    })

    await act(async () => {
      await vi.advanceTimersByTimeAsync(10000)
    })

    // Should not have been called again after unmount
    expect(invoke).toHaveBeenCalledTimes(1)

    vi.useRealTimers()
  })
})
