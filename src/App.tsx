import { useEffect, useMemo, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
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
import type { Project } from './types'

type View = 'dashboard' | 'import' | 'projects' | 'backup' | 'delivery' | 'history' | 'settings'

function App() {
  const [currentView, setCurrentView] = useState<View>('dashboard')
  const [showShortcuts, setShowShortcuts] = useState(false)
  const [selectedProjectId, setSelectedProjectId] = useState<string | null>(null)
  const [projectsCount, setProjectsCount] = useState<number>(0)

  // Apply theme on app load
  useTheme()

  // Load and refresh project count for badge
  const loadProjectCount = async () => {
    try {
      const projects = await invoke<Project[]>('list_projects')
      setProjectsCount(projects.length)
    } catch (err) {
      console.error('Failed to load project count:', err)
    }
  }

  useEffect(() => {
    loadProjectCount()
  }, [])

  // Global SD card scanner - runs in background across all pages
  const { sdCards, isScanning, scanForSDCards } = useSDCardScanner({
    onCardDetected: () => setCurrentView('import'),
  })

  // Global keyboard shortcuts - memoized to prevent re-creating on every render
  const shortcuts = useMemo(
    () => [
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
      {
        key: '6',
        metaKey: true,
        description: 'Go to History',
        action: () => setCurrentView('history'),
      },
      { key: 'r', metaKey: true, description: 'Refresh SD Cards', action: () => scanForSDCards() },
      { key: 'Escape', description: 'Close Shortcuts', action: () => setShowShortcuts(false) },
    ],
    [scanForSDCards]
  )

  useKeyboardShortcuts(shortcuts)

  const handleNavigateToProject = (projectId: string) => {
    setSelectedProjectId(projectId)
    setCurrentView('projects')
    loadProjectCount() // Refresh count after project creation
  }

  const handleViewChange = (view: string) => {
    // Clear selected project when changing views
    if (view !== 'projects') {
      setSelectedProjectId(null)
    }
    setCurrentView(view as View)
    // Refresh count when switching to views that show projects
    if (view === 'dashboard' || view === 'projects') {
      loadProjectCount()
    }
  }

  return (
    <>
      <Layout
        currentView={currentView}
        onNavigate={handleViewChange}
        importCount={sdCards.length}
        projectsCount={projectsCount}
      >
        <div style={{ display: currentView === 'dashboard' ? 'block' : 'none' }}>
          <Dashboard onProjectClick={handleNavigateToProject} />
        </div>
        <div style={{ display: currentView === 'import' ? 'block' : 'none' }}>
          <Import
            sdCards={sdCards}
            isScanning={isScanning}
            onImportComplete={handleNavigateToProject}
          />
        </div>
        <div style={{ display: currentView === 'projects' ? 'block' : 'none' }}>
          <Projects initialSelectedProjectId={selectedProjectId} />
        </div>
        <div style={{ display: currentView === 'backup' ? 'block' : 'none' }}>
          <BackupQueue />
        </div>
        <div style={{ display: currentView === 'delivery' ? 'block' : 'none' }}>
          <Delivery />
        </div>
        <div style={{ display: currentView === 'history' ? 'block' : 'none' }}>
          <History />
        </div>
        <div style={{ display: currentView === 'settings' ? 'block' : 'none' }}>
          <Settings />
        </div>
      </Layout>
      <NotificationToast />
      <KeyboardShortcutsHelp isOpen={showShortcuts} onClose={() => setShowShortcuts(false)} />
    </>
  )
}

export default App
