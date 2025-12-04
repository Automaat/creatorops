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
import { CommandPalette } from './components/CommandPalette'
import { useTheme } from './hooks/useTheme'
import { useSDCardScanner } from './hooks/useSDCardScanner'
import { useKeyboardShortcuts } from './hooks/useKeyboardShortcuts'
import type { Project } from './types'

type View = 'dashboard' | 'import' | 'projects' | 'backup' | 'delivery' | 'history' | 'settings'

function isView(value: string): value is View {
  return ['dashboard', 'import', 'projects', 'backup', 'delivery', 'history', 'settings'].includes(
    value
  )
}

function App() {
  const [currentView, setCurrentView] = useState<View>('dashboard')
  const [showShortcuts, setShowShortcuts] = useState(false)
  const [showCommandPalette, setShowCommandPalette] = useState(false)
  const [selectedProjectId, setSelectedProjectId] = useState<string | null>()
  const [projectsCount, setProjectsCount] = useState<number>(0)
  const [viewBeforeProject, setViewBeforeProject] = useState<View>('dashboard')
  const [projectsResetKey, setProjectsResetKey] = useState<number>(0)

  // Apply theme on app load
  useTheme()

  // Load and refresh project count for badge
  const loadProjectCount = async () => {
    try {
      const projects = await invoke<Project[]>('list_projects')
      setProjectsCount(projects.length)
    } catch (error) {
      console.error('Failed to load project count:', error)
    }
  }

  useEffect(() => {
    void loadProjectCount()
  }, [])

  // Global SD card scanner - runs in background across all pages
  const { sdCards, isScanning, scanForSDCards } = useSDCardScanner({
    onCardDetected: () => setCurrentView('import'),
  })

  // Global keyboard shortcuts - memoized to prevent re-creating on every render
  const shortcuts = useMemo(
    () => [
      {
        action: () => setCurrentView('settings'), description: 'Open Settings', key: ',', metaKey: true,
      },
      {
        action: () => setShowShortcuts(true), description: 'Show Keyboard Shortcuts', key: '/', metaKey: true,
      },
      {
        action: () => setShowCommandPalette(true), description: 'Open Command Palette', key: 'k', metaKey: true,
      },
      {
        action: () => setCurrentView('dashboard'), description: 'Go to Dashboard', key: '1', metaKey: true,
      },
      {
        action: () => setCurrentView('import'), description: 'Go to Import', key: '2', metaKey: true,
      },
      {
        action: () => setCurrentView('projects'), description: 'Go to Projects', key: '3', metaKey: true,
      },
      {
        action: () => setCurrentView('backup'), description: 'Go to Backup Queue', key: '4', metaKey: true,
      },
      {
        action: () => setCurrentView('delivery'), description: 'Go to Delivery', key: '5', metaKey: true,
      },
      {
        action: () => setCurrentView('history'), description: 'Go to History', key: '6', metaKey: true,
      },
      { action: () => void scanForSDCards(), description: 'Refresh SD Cards', key: 'r', metaKey: true },
      { action: () => setShowShortcuts(false), description: 'Close Shortcuts', key: 'Escape' },
    ],
    [scanForSDCards]
  )

  useKeyboardShortcuts(shortcuts)

  const handleNavigateToProject = (projectId: string) => {
    setViewBeforeProject(currentView)
    setSelectedProjectId(projectId)
    setCurrentView('projects')
    void loadProjectCount() // Refresh count after project creation
  }

  const handleBackFromProject = () => {
    setSelectedProjectId(undefined)
    setCurrentView(viewBeforeProject)
  }

  const handleViewChange = (view: string) => {
    // When navigating to Projects view via sidebar, always clear selection to show list
    if (view === 'projects') {
      setSelectedProjectId(undefined)
      // If already on projects view, force remount to reset local state
      if (currentView === 'projects') {
        setProjectsResetKey((prev) => prev + 1)
      }
    }
    if (isView(view)) {
      setCurrentView(view)
    }
    // Refresh count when switching to views that show projects
    if (view === 'dashboard' || view === 'projects') {
      void loadProjectCount()
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
          <Projects
            key={`${selectedProjectId || 'projects-list'}-${projectsResetKey}`}
            initialSelectedProjectId={selectedProjectId}
            onBackFromProject={handleBackFromProject}
          />
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
      <CommandPalette
        isOpen={showCommandPalette}
        onClose={() => setShowCommandPalette(false)}
        onSelectProject={handleNavigateToProject}
      />
    </>
  )
}

export default App
