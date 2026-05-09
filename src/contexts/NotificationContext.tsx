import type { ReactNode } from 'react'
import { useCallback, useState } from 'react'

import type { Notification, NotificationType } from './notification-context'
import { NotificationContext } from './notification-context'

const DEFAULT_NOTIFICATION_DURATION = 5000

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
