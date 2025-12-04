import React from 'react'
import ReactDOM from 'react-dom/client'

import App from './App'
import { NotificationProvider } from './contexts/NotificationContext'
import './styles/variables.css'
import './styles/global.css'
import './styles/modern.css'
import './styles/components.css'
import './styles/layouts/main-layout.css'
import './styles/notifications.css'

const rootElement = document.querySelector('#root')
if (!rootElement) {
  throw new Error('Root element not found')
}
if (!(rootElement instanceof HTMLElement)) {
  throw new Error('Root element is not an HTMLElement')
}

ReactDOM.createRoot(rootElement).render(
  <React.StrictMode>
    <NotificationProvider>
      <App />
    </NotificationProvider>
  </React.StrictMode>
)
