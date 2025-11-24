import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { render, act, renderHook } from '@testing-library/react'
import { NotificationProvider, NotificationContext } from './NotificationContext'
import { useContext } from 'react'

describe('NotificationContext', () => {
  beforeEach(() => {
    vi.useFakeTimers()
  })

  afterEach(() => {
    vi.restoreAllMocks()
    vi.useRealTimers()
  })

  it('provides notification context', () => {
    const { result } = renderHook(
      () => useContext(NotificationContext),
      { wrapper: NotificationProvider }
    )

    expect(result.current).toBeDefined()
    expect(result.current?.notifications).toEqual([])
    expect(typeof result.current?.addNotification).toBe('function')
    expect(typeof result.current?.removeNotification).toBe('function')
  })

  it('adds notification with success helper', () => {
    const { result } = renderHook(
      () => useContext(NotificationContext),
      { wrapper: NotificationProvider }
    )

    act(() => {
      result.current?.success('Test success')
    })

    expect(result.current?.notifications).toHaveLength(1)
    expect(result.current?.notifications[0].type).toBe('success')
    expect(result.current?.notifications[0].message).toBe('Test success')
  })

  it('adds notification with error helper', () => {
    const { result } = renderHook(
      () => useContext(NotificationContext),
      { wrapper: NotificationProvider }
    )

    act(() => {
      result.current?.error('Test error')
    })

    expect(result.current?.notifications).toHaveLength(1)
    expect(result.current?.notifications[0].type).toBe('error')
    expect(result.current?.notifications[0].message).toBe('Test error')
  })

  it('adds notification with warning helper', () => {
    const { result } = renderHook(
      () => useContext(NotificationContext),
      { wrapper: NotificationProvider }
    )

    act(() => {
      result.current?.warning('Test warning')
    })

    expect(result.current?.notifications).toHaveLength(1)
    expect(result.current?.notifications[0].type).toBe('warning')
    expect(result.current?.notifications[0].message).toBe('Test warning')
  })

  it('adds notification with info helper', () => {
    const { result } = renderHook(
      () => useContext(NotificationContext),
      { wrapper: NotificationProvider }
    )

    act(() => {
      result.current?.info('Test info')
    })

    expect(result.current?.notifications).toHaveLength(1)
    expect(result.current?.notifications[0].type).toBe('info')
    expect(result.current?.notifications[0].message).toBe('Test info')
  })

  it('removes notification manually', () => {
    const { result } = renderHook(
      () => useContext(NotificationContext),
      { wrapper: NotificationProvider }
    )

    act(() => {
      result.current?.success('Test')
    })

    const id = result.current?.notifications[0].id

    act(() => {
      result.current?.removeNotification(id!)
    })

    expect(result.current?.notifications).toHaveLength(0)
  })

  it('auto-removes notification after duration', async () => {
    const { result } = renderHook(
      () => useContext(NotificationContext),
      { wrapper: NotificationProvider }
    )

    act(() => {
      result.current?.success('Test', 1000)
    })

    expect(result.current?.notifications).toHaveLength(1)

    await act(async () => {
      vi.advanceTimersByTime(1000)
      await vi.runAllTimersAsync()
    })

    expect(result.current?.notifications).toHaveLength(0)
  })

  it('supports custom duration', async () => {
    const { result } = renderHook(
      () => useContext(NotificationContext),
      { wrapper: NotificationProvider }
    )

    act(() => {
      result.current?.success('Test', 2000)
    })

    act(() => {
      vi.advanceTimersByTime(1000)
    })

    expect(result.current?.notifications).toHaveLength(1)

    await act(async () => {
      vi.advanceTimersByTime(1000)
      await vi.runAllTimersAsync()
    })

    expect(result.current?.notifications).toHaveLength(0)
  })

  it('handles multiple notifications', () => {
    const { result } = renderHook(
      () => useContext(NotificationContext),
      { wrapper: NotificationProvider }
    )

    act(() => {
      result.current?.success('First')
      result.current?.error('Second')
      result.current?.warning('Third')
    })

    expect(result.current?.notifications).toHaveLength(3)
    expect(result.current?.notifications[0].message).toBe('First')
    expect(result.current?.notifications[1].message).toBe('Second')
    expect(result.current?.notifications[2].message).toBe('Third')
  })

  it('generates unique IDs for notifications', () => {
    const { result } = renderHook(
      () => useContext(NotificationContext),
      { wrapper: NotificationProvider }
    )

    act(() => {
      result.current?.success('First')
      result.current?.success('Second')
    })

    const ids = result.current?.notifications.map((n) => n.id)
    expect(new Set(ids).size).toBe(2)
  })

  it('renders children correctly', () => {
    const { getByText } = render(
      <NotificationProvider>
        <div>Test Child</div>
      </NotificationProvider>
    )

    expect(getByText('Test Child')).toBeTruthy()
  })
})
