import { useCallback, useEffect, useMemo, useState } from 'react'
import type { ReactNode } from 'react'
import { invoke } from '@tauri-apps/api/core'

import { CommandPalette } from './components/CommandPalette'
import { Clients } from './components/Clients'
import { Dashboard } from './components/Dashboard'
import { BackupQueue } from './components/BackupQueue'
import { Delivery } from './components/Delivery'
import { ErrorBoundary } from './components/ErrorBoundary'
import { History } from './components/History'
import { Import } from './components/Import'
import { KeyboardShortcutsHelp } from './components/KeyboardShortcutsHelp'
import { Layout } from './components/Layout'
import { NotificationToast } from './components/NotificationToast'
import { Projects } from './components/Projects'
import { Settings } from './components/Settings'
import { useKeyboardShortcuts } from './hooks/useKeyboardShortcuts'
import { useNotification } from './hooks/useNotification'
import { useSDCardScanner } from './hooks/useSDCardScanner'
import { useTheme } from './hooks/useTheme'
import type { Client, Project } from './types'

type View =
  | 'dashboard'
  | 'import'
  | 'projects'
  | 'clients'
  | 'backup'
  | 'delivery'
  | 'history'
  | 'settings'

function isView(value: string): value is View {
  return [
    'dashboard',
    'import',
    'projects',
    'clients',
    'backup',
    'delivery',
    'history',
    'settings',
  ].includes(value)
}

interface ViewWrapperProps {
  isActive: boolean
  name: string
  children: ReactNode
}

function ViewWrapper({ isActive, name, children }: ViewWrapperProps) {
  const className = isActive ? 'view-active' : 'view-hidden'
  return (
    <div className={className}>
      <ErrorBoundary name={name} isActive={isActive}>
        {children}
      </ErrorBoundary>
    </div>
  )
}

function App() {
  const [currentView, setCurrentView] = useState<View>('dashboard')
  const [showShortcuts, setShowShortcuts] = useState(false)
  const [showCommandPalette, setShowCommandPalette] = useState(false)
  const [selectedProjectId, setSelectedProjectId] = useState<string | null>()
  const [projectsCount, setProjectsCount] = useState<number>(0)
  const [clientsCount, setClientsCount] = useState<number>(0)
  const [viewBeforeProject, setViewBeforeProject] = useState<View>('dashboard')
  const [projectsResetKey, setProjectsResetKey] = useState<number>(0)

  const { error: showError } = useNotification()

  // Apply theme on app load
  useTheme()

  // Load and refresh project count for badge
  const loadProjectCount = useCallback(async () => {
    try {
      const projects = await invoke<Project[]>('list_projects')
      setProjectsCount(projects.length)
    } catch (error) {
      console.error('Failed to load project count:', error)
      showError('Failed to load project count')
    }
  }, [showError])

  const loadClientsCount = useCallback(async () => {
    try {
      const clients = await invoke<Client[]>('list_clients', { includeArchived: false })
      setClientsCount(clients.length)
    } catch (error) {
      console.error('Failed to load client count:', error)
    }
  }, [])

  useEffect(() => {
    void loadProjectCount()
    void loadClientsCount()
  }, [loadProjectCount, loadClientsCount])

  // Global SD card scanner - runs in background across all pages
  const { sdCards, isScanning, scanForSDCards } = useSDCardScanner({
    onCardDetected: () => setCurrentView('import'),
  })

  // Global keyboard shortcuts - memoized to prevent re-creating on every render
  const shortcuts = useMemo(
    () => [
      {
        action: () => setCurrentView('settings'),
        description: 'Open Settings',
        key: ',',
        metaKey: true,
      },
      {
        action: () => setShowShortcuts(true),
        description: 'Show Keyboard Shortcuts',
        key: '/',
        metaKey: true,
      },
      {
        action: () => setShowCommandPalette(true),
        description: 'Open Command Palette',
        key: 'k',
        metaKey: true,
      },
      {
        action: () => setCurrentView('dashboard'),
        description: 'Go to Dashboard',
        key: '1',
        metaKey: true,
      },
      {
        action: () => setCurrentView('import'),
        description: 'Go to Import',
        key: '2',
        metaKey: true,
      },
      {
        action: () => setCurrentView('projects'),
        description: 'Go to Projects',
        key: '3',
        metaKey: true,
      },
      {
        action: () => setCurrentView('clients'),
        description: 'Go to Clients',
        key: '4',
        metaKey: true,
      },
      {
        action: () => setCurrentView('backup'),
        description: 'Go to Backup Queue',
        key: '5',
        metaKey: true,
      },
      {
        action: () => setCurrentView('delivery'),
        description: 'Go to Delivery',
        key: '6',
        metaKey: true,
      },
      {
        action: () => setCurrentView('history'),
        description: 'Go to History',
        key: '7',
        metaKey: true,
      },
      {
        action: () => void scanForSDCards(),
        description: 'Refresh SD Cards',
        key: 'r',
        metaKey: true,
      },
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
    // Refresh counts when switching to relevant views
    if (view === 'dashboard' || view === 'projects') {
      void loadProjectCount()
    }
    if (view === 'clients') {
      void loadClientsCount()
    }
  }

  return (
    <>
      <Layout
        currentView={currentView}
        onNavigate={handleViewChange}
        importCount={sdCards.length}
        projectsCount={projectsCount}
        clientsCount={clientsCount}
      >
        <ViewWrapper isActive={currentView === 'dashboard'} name="Dashboard">
          <Dashboard
            isActive={currentView === 'dashboard'}
            onProjectClick={handleNavigateToProject}
          />
        </ViewWrapper>
        <ViewWrapper isActive={currentView === 'import'} name="Import">
          <Import
            sdCards={sdCards}
            isScanning={isScanning}
            onImportComplete={handleNavigateToProject}
          />
        </ViewWrapper>
        <ViewWrapper isActive={currentView === 'projects'} name="Projects">
          <Projects
            key={`${selectedProjectId ?? 'projects-list'}-${projectsResetKey}`}
            initialSelectedProjectId={selectedProjectId}
            isActive={currentView === 'projects'}
            onBackFromProject={handleBackFromProject}
          />
        </ViewWrapper>
        <ViewWrapper isActive={currentView === 'clients'} name="Clients">
          <Clients isActive={currentView === 'clients'} />
        </ViewWrapper>
        <ViewWrapper isActive={currentView === 'backup'} name="Backup Queue">
          <BackupQueue isActive={currentView === 'backup'} />
        </ViewWrapper>
        <ViewWrapper isActive={currentView === 'delivery'} name="Delivery">
          <Delivery isActive={currentView === 'delivery'} />
        </ViewWrapper>
        <ViewWrapper isActive={currentView === 'history'} name="History">
          <History />
        </ViewWrapper>
        <ViewWrapper isActive={currentView === 'settings'} name="Settings">
          <Settings isActive={currentView === 'settings'} />
        </ViewWrapper>
      </Layout>
      <NotificationToast />
      <KeyboardShortcutsHelp
        isOpen={showShortcuts}
        onClose={() => {
          setShowShortcuts(false)
        }}
      />
      <CommandPalette
        isOpen={showCommandPalette}
        onClose={() => {
          setShowCommandPalette(false)
        }}
        onSelectProject={handleNavigateToProject}
      />
    </>
  )
}

export default App
