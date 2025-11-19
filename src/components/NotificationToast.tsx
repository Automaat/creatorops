import { useNotification } from '../hooks/useNotification'
import '../styles/notifications.css'

export function NotificationToast() {
  const { notifications, removeNotification } = useNotification()

  return (
    <div className="notification-container">
      {notifications.map((notification) => (
        <div
          key={notification.id}
          className={`notification notification-${notification.type}`}
          onClick={() => removeNotification(notification.id)}
        >
          <div className="notification-content">
            <span className="notification-icon">{getIcon(notification.type)}</span>
            <span className="notification-message">{notification.message}</span>
          </div>
          <button
            className="notification-close"
            onClick={(e) => {
              e.stopPropagation()
              removeNotification(notification.id)
            }}
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
    case 'success':
      return '✓'
    case 'error':
      return '✕'
    case 'warning':
      return '⚠'
    case 'info':
      return 'ℹ'
    default:
      return ''
  }
}
