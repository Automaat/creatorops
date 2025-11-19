import { useState } from 'react'
import { Layout } from './components/Layout'
import { Dashboard } from './components/Dashboard'
import { Import } from './components/Import'
import { Projects } from './components/Projects'
import { BackupQueue } from './components/BackupQueue'
import { Delivery } from './components/Delivery'
import { History } from './components/History'
import { Settings } from './components/Settings'
import { NotificationToast } from './components/NotificationToast'
import { KeyboardShortcutsHelp } from './components/KeyboardShortcutsHelp'
import { useTheme } from './hooks/useTheme'
import { useSDCardScanner } from './hooks/useSDCardScanner'
import { useKeyboardShortcuts } from './hooks/useKeyboardShortcuts'

type View = 'dashboard' | 'import' | 'projects' | 'backup' | 'delivery' | 'history' | 'settings'

function App() {
  const [currentView, setCurrentView] = useState<View>('dashboard')
  const [showShortcuts, setShowShortcuts] = useState(false)

  // Apply theme on app load
  useTheme()

  // Global SD card scanner - runs in background across all pages
  const { sdCards, isScanning, scanForSDCards } = useSDCardScanner()

  // Global keyboard shortcuts
  useKeyboardShortcuts([
    {
      key: ',',
      metaKey: true,
      description: 'Open Settings',
      action: () => setCurrentView('settings'),
    },
    {
      key: '/',
      metaKey: true,
      description: 'Show Keyboard Shortcuts',
      action: () => setShowShortcuts(true),
    },
    {
      key: '1',
      metaKey: true,
      description: 'Go to Dashboard',
      action: () => setCurrentView('dashboard'),
    },
    {
      key: '2',
      metaKey: true,
      description: 'Go to Import',
      action: () => setCurrentView('import'),
    },
    {
      key: '3',
      metaKey: true,
      description: 'Go to Projects',
      action: () => setCurrentView('projects'),
    },
    {
      key: '4',
      metaKey: true,
      description: 'Go to Backup Queue',
      action: () => setCurrentView('backup'),
    },
    {
      key: '5',
      metaKey: true,
      description: 'Go to Delivery',
      action: () => setCurrentView('delivery'),
    },
    { key: 'r', metaKey: true, description: 'Refresh SD Cards', action: () => scanForSDCards() },
    { key: 'Escape', description: 'Close Shortcuts', action: () => setShowShortcuts(false) },
  ])

  return (
    <>
      <Layout currentView={currentView} onNavigate={(view) => setCurrentView(view as View)}>
        {currentView === 'dashboard' && <Dashboard />}
        {currentView === 'import' && (
          <Import sdCards={sdCards} isScanning={isScanning} onRefresh={scanForSDCards} />
        )}
        {currentView === 'projects' && <Projects />}
        {currentView === 'backup' && <BackupQueue />}
        {currentView === 'delivery' && <Delivery />}
        {currentView === 'history' && <History />}
        {currentView === 'settings' && <Settings />}
      </Layout>
      <NotificationToast />
      <KeyboardShortcutsHelp isOpen={showShortcuts} onClose={() => setShowShortcuts(false)} />
    </>
  )
}

export default App
