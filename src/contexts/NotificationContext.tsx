/* eslint-disable react-refresh/only-export-components */
import type { ReactNode } from 'react'
import { createContext, useCallback, useState } from 'react'

const DEFAULT_NOTIFICATION_DURATION = 5000

export type NotificationType = 'success' | 'error' | 'warning' | 'info'

export interface Notification {
  id: string
  type: NotificationType
  message: string
  duration?: number
}

interface NotificationContextType {
  notifications: Notification[]
  addNotification: (type: NotificationType, message: string, duration?: number) => void
  removeNotification: (id: string) => void
  success: (message: string, duration?: number) => void
  error: (message: string, duration?: number) => void
  warning: (message: string, duration?: number) => void
  info: (message: string, duration?: number) => void
}

export const NotificationContext = createContext<NotificationContextType | undefined>(undefined)

export function NotificationProvider({ children }: { children: ReactNode }) {
  const [notifications, setNotifications] = useState<Notification[]>([])

  const removeNotification = useCallback((id: string) => {
    setNotifications((prev) => prev.filter((n) => n.id !== id))
  }, [])

  const addNotification = useCallback(
    (type: NotificationType, message: string, duration = DEFAULT_NOTIFICATION_DURATION) => {
      const id = crypto.randomUUID()
      const notification: Notification = { duration, id, message, type }

      setNotifications((prev) => [...prev, notification])

      if (duration > 0) {
        setTimeout(() => {
          removeNotification(id)
        }, duration)
      }
    },
    [removeNotification]
  )

  const success = useCallback(
    (message: string, duration?: number) => addNotification('success', message, duration),
    [addNotification]
  )

  const error = useCallback(
    (message: string, duration?: number) => addNotification('error', message, duration),
    [addNotification]
  )

  const warning = useCallback(
    (message: string, duration?: number) => addNotification('warning', message, duration),
    [addNotification]
  )

  const info = useCallback(
    (message: string, duration?: number) => addNotification('info', message, duration),
    [addNotification]
  )

  return (
    <NotificationContext.Provider
      value={{
        addNotification,
        error,
        info,
        notifications,
        removeNotification,
        success,
        warning,
      }}
    >
      {children}
    </NotificationContext.Provider>
  )
}
