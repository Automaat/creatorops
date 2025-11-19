import { useState, useEffect } from 'react'
import { Layout } from './components/Layout'
import { Dashboard } from './components/Dashboard'
import { Import } from './components/Import'
import { Projects } from './components/Projects'
import { BackupQueue } from './components/BackupQueue'
import { Settings } from './components/Settings'
import { useTheme } from './hooks/useTheme'
import { useSDCardScanner } from './hooks/useSDCardScanner'

type View = 'dashboard' | 'import' | 'projects' | 'backup' | 'settings'

function App() {
  const [currentView, setCurrentView] = useState<View>('dashboard')

  // Apply theme on app load
  useTheme()

  // Global SD card scanner - runs in background across all pages
  const { sdCards, isScanning, scanForSDCards } = useSDCardScanner()

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
      {currentView === 'import' && (
        <Import sdCards={sdCards} isScanning={isScanning} onRefresh={scanForSDCards} />
      )}
      {currentView === 'projects' && <Projects />}
      {currentView === 'backup' && <BackupQueue />}
      {currentView === 'settings' && <Settings />}
    </Layout>
  )
}

export default App
