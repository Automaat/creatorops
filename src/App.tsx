import { useState, useEffect } from 'react'
import { Layout } from './components/Layout'
import { Dashboard } from './components/Dashboard'
import { Import } from './components/Import'
import { Settings } from './components/Settings'
import { useTheme } from './hooks/useTheme'

type View = 'dashboard' | 'import' | 'projects' | 'backup' | 'settings'

function App() {
  const [currentView, setCurrentView] = useState<View>('dashboard')

  // Apply theme on app load
  useTheme()

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Cmd+, or Ctrl+, to open settings (classic macOS shortcut)
      if ((e.metaKey || e.ctrlKey) && e.key === ',') {
        e.preventDefault()
        setCurrentView('settings')
      }
    }

    window.addEventListener('keydown', handleKeyDown)
    return () => window.removeEventListener('keydown', handleKeyDown)
  }, [])

  return (
    <Layout currentView={currentView} onNavigate={(view) => setCurrentView(view as View)}>
      {currentView === 'dashboard' && <Dashboard />}
      {currentView === 'import' && <Import />}
      {currentView === 'settings' && <Settings />}
      {currentView === 'projects' && (
        <div className="content-body">
          <div className="card">
            <p className="text-secondary">Projects view coming soon</p>
          </div>
        </div>
      )}
      {currentView === 'backup' && (
        <div className="content-body">
          <div className="card">
            <p className="text-secondary">Backup queue coming soon</p>
          </div>
        </div>
      )}
    </Layout>
  )
}

export default App
