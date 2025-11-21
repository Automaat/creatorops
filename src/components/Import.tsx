import { useEffect, useState, useRef } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type { SDCard, Project, CopyResult, ImportProgress } from '../types'
import { ProjectStatus } from '../types'
import { CreateProject } from './CreateProject'
import { sortProjectsByStatus } from '../utils/project'

const POST_IMPORT_DELAY_MS = 1500 // Allow user to see success message

interface ImportProps {
  sdCards: SDCard[]
  isScanning: boolean
  onImportComplete: (projectId: string) => void
}

export function Import({ sdCards, isScanning, onImportComplete }: ImportProps) {
  const [activeCardPath, setActiveCardPath] = useState<string | null>(null)
  const listRef = useRef<HTMLDivElement>(null)

  // Reset active card when the active card is no longer in the list
  useEffect(() => {
    if (activeCardPath && !sdCards.some((card) => card.path === activeCardPath)) {
      setActiveCardPath(null)
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [sdCards.length, activeCardPath])

  // Handle clicks outside the list to collapse active card
  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (
        activeCardPath &&
        listRef.current &&
        !listRef.current.contains(event.target as Node)
      ) {
        setActiveCardPath(null)
      }
    }

    document.addEventListener('mousedown', handleClickOutside)
    return () => {
      document.removeEventListener('mousedown', handleClickOutside)
    }
  }, [activeCardPath])

  return (
    <>
      <div className="content-header">
        <div>
          <h1>Import from SD Card</h1>
          <p className="text-secondary">Detect and import files from SD cards</p>
        </div>
      </div>
      <div className="content-body">
        {sdCards.length === 0 ? (
          <div className="card">
            <p className="text-secondary">
              {isScanning
                ? 'Scanning for SD cards...'
                : 'No SD cards detected. Insert an SD card and click Refresh.'}
            </p>
          </div>
        ) : (
          <div ref={listRef} className="project-list">
            {sdCards.map((card) => (
              <SDCardItem
                key={card.path}
                card={card}
                onImportComplete={onImportComplete}
                isActive={activeCardPath === card.path}
                onActivate={() => setActiveCardPath(card.path)}
                onDeactivate={() => setActiveCardPath(null)}
              />
            ))}
          </div>
        )}
      </div>
    </>
  )
}

interface SDCardItemProps {
  card: SDCard
  onImportComplete: (projectId: string) => void
  isActive: boolean
  onActivate: () => void
  onDeactivate: () => void
}

function SDCardItem({
  card,
  onImportComplete,
  isActive,
  onActivate,
  onDeactivate,
}: SDCardItemProps) {
  const [showProjectSelect, setShowProjectSelect] = useState(false)
  const [projects, setProjects] = useState<Project[]>([])
  const [selectedProject, setSelectedProject] = useState<string>('')
  const [showCreateNew, setShowCreateNew] = useState(false)
  const [isImporting, setIsImporting] = useState(false)
  const [importResult, setImportResult] = useState<CopyResult | null>(null)
  const [importId, setImportId] = useState<string | null>(null)
  const [importProgress, setImportProgress] = useState<ImportProgress | null>(null)
  const [dropdownPosition, setDropdownPosition] = useState<{
    top: number
    left: number
    width: number
  } | null>(null)
  const triggerRef = useRef<HTMLButtonElement>(null)
  const dropdownRef = useRef<HTMLDivElement>(null)

  function getStatusColor(status: string): string {
    switch (status) {
      case 'New':
        return 'status-new'
      case 'Importing':
        return 'status-importing'
      case 'Editing':
        return 'status-editing'
      case 'Delivered':
        return 'status-delivered'
      case 'Archived':
        return 'status-archived'
      default:
        return ''
    }
  }

  useEffect(() => {
    const unlistenProgress = listen<ImportProgress>('import-progress', (event) => {
      setImportProgress(event.payload)
    })

    return () => {
      unlistenProgress.then((fn) => fn())
    }
  }, [])

  useEffect(() => {
    if (showProjectSelect) {
      loadProjects()
      updateDropdownPosition()
    }
  }, [showProjectSelect])

  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (
        showProjectSelect &&
        triggerRef.current &&
        dropdownRef.current &&
        !triggerRef.current.contains(event.target as Node) &&
        !dropdownRef.current.contains(event.target as Node)
      ) {
        setShowProjectSelect(false)
      }
    }

    if (showProjectSelect) {
      document.addEventListener('mousedown', handleClickOutside)
    }

    return () => {
      document.removeEventListener('mousedown', handleClickOutside)
    }
  }, [showProjectSelect])

  const updateDropdownPosition = () => {
    if (triggerRef.current) {
      const rect = triggerRef.current.getBoundingClientRect()
      setDropdownPosition({
        top: rect.bottom + 8,
        left: rect.left,
        width: rect.width,
      })
    }
  }

  const loadProjects = async () => {
    try {
      const projectList = await invoke<Project[]>('list_projects')
      const sortedProjects = sortProjectsByStatus(projectList)
      setProjects(sortedProjects)
    } catch (error) {
      console.error('Failed to load projects:', error)
    }
  }

  const handleImportClick = () => {
    onActivate()
  }

  const handleProjectSelect = (projectId: string) => {
    if (projectId === '__new__') {
      setShowCreateNew(true)
    } else {
      setSelectedProject(projectId)
    }
  }

  const handleProjectCreated = (project: Project) => {
    setShowCreateNew(false)
    setSelectedProject(project.id)
    const updatedProjects = sortProjectsByStatus([project, ...projects])
    setProjects(updatedProjects)
  }

  const handleStartImport = async () => {
    if (!selectedProject) return

    const project = projects.find((p) => p.id === selectedProject)
    if (!project) return

    // Generate unique import ID
    const currentImportId = `import-${Date.now()}-${Math.random().toString(36).substring(7)}`
    setImportId(currentImportId)
    setIsImporting(true)
    setImportResult(null)
    setImportProgress(null)

    const startedAt = new Date().toISOString()

    // Update project status to Importing
    // Note: If this fails, we still proceed with import - status update is non-critical
    try {
      await invoke('update_project_status', {
        projectId: project.id,
        newStatus: ProjectStatus.Importing,
      })
    } catch (err) {
      console.error('Failed to update project status:', err)
      // Continue with import - status update failure is not critical
    }

    try {
      // Get all photo/video files from the SD card
      const sourcePaths = await invoke<string[]>('list_sd_card_files', {
        cardPath: card.path,
      })

      if (sourcePaths.length === 0) {
        const result = {
          success: false,
          error: 'No photo or video files found on SD card',
          filesCopied: 0,
          filesSkipped: 0,
          skippedFiles: [],
          totalBytes: 0,
          photosCopied: 0,
          videosCopied: 0,
        }
        setImportResult(result)
        setIsImporting(false)
        setImportId(null)

        // Save to history
        await invoke('save_import_history', {
          projectId: project.id,
          projectName: project.name,
          sourcePath: card.path,
          destinationPath: `${project.folderPath}/RAW`,
          filesCopied: 0,
          filesSkipped: 0,
          totalBytes: 0,
          photosCopied: 0,
          videosCopied: 0,
          startedAt,
          errorMessage: result.error,
        })

        return
      }

      const destination = `${project.folderPath}/RAW`

      const result = await invoke<CopyResult>('copy_files', {
        importId: currentImportId,
        sourcePaths,
        destination,
      })

      setImportResult(result)

      // Check if cancelled
      const wasCancelled = result.error?.includes('cancelled') || false

      // Save to history (unless cancelled)
      if (!wasCancelled) {
        await invoke('save_import_history', {
          projectId: project.id,
          projectName: project.name,
          sourcePath: card.path,
          destinationPath: destination,
          filesCopied: result.filesCopied,
          filesSkipped: result.filesSkipped,
          totalBytes: result.totalBytes,
          photosCopied: result.photosCopied,
          videosCopied: result.videosCopied,
          startedAt,
          errorMessage: result.error || null,
        })
      }

      // Navigate to project view on successful import
      if (result.success && result.filesCopied > 0) {
        setTimeout(() => onImportComplete(project.id), POST_IMPORT_DELAY_MS)
      }
    } catch (error) {
      console.error('Import failed:', error)
      setImportResult({
        success: false,
        error: String(error),
        filesCopied: 0,
        filesSkipped: 0,
        skippedFiles: [],
        totalBytes: 0,
        photosCopied: 0,
        videosCopied: 0,
      })

      // Save failed import to history
      try {
        await invoke('save_import_history', {
          projectId: project.id,
          projectName: project.name,
          sourcePath: card.path,
          destinationPath: `${project.folderPath}/RAW`,
          filesCopied: 0,
          filesSkipped: 0,
          totalBytes: 0,
          photosCopied: 0,
          videosCopied: 0,
          startedAt,
          errorMessage: String(error),
        })
      } catch (historyError) {
        console.error('Failed to save import history:', historyError)
      }
    } finally {
      setIsImporting(false)
      setImportId(null)
    }
  }

  const handleCancelImport = async () => {
    if (!importId) return

    try {
      await invoke('cancel_import', { importId })
    } catch (error) {
      console.error('Failed to cancel import:', error)
    }
  }

  // Collapsed card view - click to expand
  if (!isActive) {
    return (
      <div className="project-list-item clickable" onClick={handleImportClick}>
        <div className="project-list-content">
          <div>
            <h3>{card.name}</h3>
            <p className="text-secondary text-sm">Click to import</p>
          </div>
        </div>
      </div>
    )
  }

  // Expanded card view - show project dropdown
  if (isActive && !isImporting && !importResult) {
    const selectedProjectData = projects.find((p) => p.id === selectedProject)

    return (
      <>
        <div className="project-list-item">
          <div className="card card-active" style={{ margin: '16px', borderRadius: 'var(--radius-lg)' }}>
            <div className="flex flex-col gap-md">
              <div>
                <h3>{card.name}</h3>
                <p className="text-secondary text-sm">Select a project to import into</p>
              </div>

            <div className="project-dropdown-container">
              <button
                ref={triggerRef}
                className="project-dropdown-trigger"
                onClick={() => setShowProjectSelect(!showProjectSelect)}
              >
                {selectedProjectData ? (
                  <div className="project-dropdown-selected">
                    <div className="project-select-header">
                      <h4>{selectedProjectData.name}</h4>
                      <span
                        className={`project-status ${getStatusColor(selectedProjectData.status)}`}
                      >
                        {selectedProjectData.status}
                      </span>
                    </div>
                    <div className="project-select-info">
                      <span className="text-secondary text-sm">
                        {selectedProjectData.clientName} · {selectedProjectData.date} ·{' '}
                        {selectedProjectData.shootType}
                      </span>
                    </div>
                  </div>
                ) : (
                  <span className="text-secondary">Choose a project...</span>
                )}
                <span className="dropdown-arrow">{showProjectSelect ? '▲' : '▼'}</span>
              </button>

              {showProjectSelect && dropdownPosition && (
                <div
                  ref={dropdownRef}
                  className="project-dropdown-list project-dropdown-list-fixed"
                  style={{
                    top: `${dropdownPosition.top}px`,
                    left: `${dropdownPosition.left}px`,
                    width: `${dropdownPosition.width}px`,
                  }}
                >
                  {projects.length > 0 ? (
                    <>
                      {projects.map((project) => (
                        <div
                          key={project.id}
                          className={`project-select-card ${selectedProject === project.id ? 'selected' : ''}`}
                          onClick={() => {
                            setSelectedProject(project.id)
                            setShowProjectSelect(false)
                          }}
                        >
                          <div className="project-select-header">
                            <h4>{project.name}</h4>
                            <span className={`project-status ${getStatusColor(project.status)}`}>
                              {project.status}
                            </span>
                          </div>
                          <div className="project-select-info">
                            <span className="text-secondary text-sm">
                              {project.clientName} · {project.date} · {project.shootType}
                            </span>
                          </div>
                        </div>
                      ))}
                      <div
                        className="project-select-card create-new"
                        onClick={() => {
                          handleProjectSelect('__new__')
                          setShowProjectSelect(false)
                        }}
                      >
                        <div className="project-select-header">
                          <h4>+ Create New Project</h4>
                        </div>
                      </div>
                    </>
                  ) : (
                    <div className="empty-state">
                      <p className="text-secondary">No projects available</p>
                      <button
                        className="btn btn-primary"
                        onClick={() => {
                          setShowCreateNew(true)
                          setShowProjectSelect(false)
                        }}
                      >
                        Create New Project
                      </button>
                    </div>
                  )}
                </div>
              )}
            </div>

              <div className="flex gap-sm" style={{ marginTop: 'var(--space-sm)' }}>
                <button
                  className="btn btn-primary"
                  onClick={handleStartImport}
                  disabled={!selectedProject || selectedProject === '__new__'}
                >
                  Start Import
                </button>
                <button
                  className="btn"
                  onClick={() => {
                    setSelectedProject('')
                    setShowProjectSelect(false)
                    onDeactivate()
                  }}
                >
                  Cancel
                </button>
              </div>
            </div>
          </div>
        </div>

        {showCreateNew && (
          <div className="dialog-overlay" onClick={() => setShowCreateNew(false)}>
            <div className="dialog" onClick={(e) => e.stopPropagation()}>
              <h2>Create New Project</h2>
              <CreateProject
                onProjectCreated={handleProjectCreated}
                onCancel={() => setShowCreateNew(false)}
              />
            </div>
          </div>
        )}
      </>
    )
  }

  if (isImporting) {
    const percentage = importProgress
      ? (importProgress.filesCopied / importProgress.totalFiles) * 100
      : 0

    return (
      <div className="project-list-item">
        <div className="card" style={{ margin: '16px', borderRadius: 'var(--radius-lg)' }}>
          <div className="flex flex-col gap-md">
            <div>
              <h3>{card.name}</h3>
              <p className="text-secondary text-sm">Importing files...</p>
            </div>

            {importProgress && (
              <div className="backup-progress">
                <div className="progress-info">
                  <span className="progress-file">{importProgress.currentFile}</span>
                  <span className="progress-count">
                    {importProgress.filesCopied} / {importProgress.totalFiles} files
                  </span>
                </div>

                <div className="progress-bar">
                  <div className="progress-fill" style={{ width: `${percentage}%` }} />
                </div>
              </div>
            )}

            <button className="btn" onClick={handleCancelImport}>
              Cancel Import
            </button>
          </div>
        </div>
      </div>
    )
  }

  if (importResult) {
    const wasCancelled = importResult.error?.includes('cancelled') || false

    return (
      <div className="project-list-item">
        <div className="card" style={{ margin: '16px', borderRadius: 'var(--radius-lg)' }}>
          <div className="flex flex-col gap-md">
            <div>
              <h3>{card.name}</h3>
              <p
                className={`text-sm ${wasCancelled ? 'text-warning' : importResult.success ? 'text-success' : 'text-error'}`}
              >
                {wasCancelled
                  ? `Import cancelled (${importResult.filesCopied} files copied)`
                  : importResult.success
                    ? 'Import completed'
                    : 'Import failed'}
              </p>
            </div>

            {!wasCancelled && (
              <div className="text-sm">
                <p>Files copied: {importResult.filesCopied}</p>
                {importResult.photosCopied > 0 && <p>Photos: {importResult.photosCopied}</p>}
                {importResult.videosCopied > 0 && <p>Videos: {importResult.videosCopied}</p>}
                {importResult.filesSkipped > 0 && (
                  <>
                    <p className="text-warning">Files skipped: {importResult.filesSkipped}</p>
                    {importResult.skippedFiles.length > 0 && (
                      <div className="mt-xs">
                        <p className="font-medium">Skipped files:</p>
                        <ul className="list-disc ml-md">
                          {importResult.skippedFiles.map((file, i) => (
                            <li key={i}>{file}</li>
                          ))}
                        </ul>
                      </div>
                    )}
                  </>
                )}
                {importResult.error && <p className="text-error mt-xs">{importResult.error}</p>}
              </div>
            )}

            <button className="btn" onClick={() => setImportResult(null)}>
              Done
            </button>
          </div>
        </div>
      </div>
    )
  }

  // This should never be reached, but just in case return null
  return null
}
