import { useNotification } from '../hooks/useNotification'
import '../styles/notifications.css'

export function NotificationToast() {
  const { notifications, removeNotification } = useNotification()

  const handleNotificationClick = (id: string) => {
    removeNotification(id)
  }

  const handleNotificationKeyDown = (e: React.KeyboardEvent, id: string) => {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault()
      removeNotification(id)
    }
  }

  const handleCloseClick = (e: React.MouseEvent, id: string) => {
    e.stopPropagation()
    removeNotification(id)
  }

  return (
    <div className="notification-container">
      {notifications.map((notification) => (
        <div
          key={notification.id}
          className={`notification notification-${notification.type}`}
          onClick={() => handleNotificationClick(notification.id)}
          onKeyDown={(e) => handleNotificationKeyDown(e, notification.id)}
          role="button"
          tabIndex={0}
        >
          <div className="notification-content">
            <span className="notification-icon">{getIcon(notification.type)}</span>
            <span className="notification-message">{notification.message}</span>
          </div>
          <button
            type="button"
            className="notification-close"
            onClick={(e) => handleCloseClick(e, notification.id)}
            aria-label="Dismiss notification"
          >
            ×
          </button>
        </div>
      ))}
    </div>
  )
}

function getIcon(type: string): string {
  switch (type) {
    case 'success': {
      return '✓'
    }
    case 'error': {
      return '✕'
    }
    case 'warning': {
      return '⚠'
    }
    case 'info': {
      return 'ℹ'
    }
    default: {
      return ''
    }
  }
}
