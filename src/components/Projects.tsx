import { useCallback, useEffect, useRef, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import type {
  Project,
  BackupDestination,
  ArchiveJob,
  ImportHistory,
  SDCard,
  CopyResult,
} from '../types'
import { ProjectStatus } from '../types'
import { CreateProject } from './CreateProject'
import { useSDCardScanner } from '../hooks/useSDCardScanner'
import { DatePicker } from './DatePicker'
import { formatDisplayDate } from '../utils/formatting'
import folderIcon from '../assets/icons/dir_selected.png'

interface ProjectsProps {
  initialSelectedProjectId?: string | null
  onBackFromProject?: () => void
}

export function Projects({ initialSelectedProjectId, onBackFromProject }: ProjectsProps) {
  const [projects, setProjects] = useState<Project[]>([])
  const [destinations, setDestinations] = useState<BackupDestination[]>([])
  const [selectedProject, setSelectedProject] = useState<Project | null>(null)
  const [loading, setLoading] = useState(true)
  const [archiveLocation, setArchiveLocation] = useState('')
  const [showArchiveDialog, setShowArchiveDialog] = useState(false)
  const [showCreateProject, setShowCreateProject] = useState(false)
  const [showDeleteDialog, setShowDeleteDialog] = useState(false)
  const [isDeleting, setIsDeleting] = useState(false)
  const [importHistory, setImportHistory] = useState<ImportHistory[]>([])
  const [showImportDialog, setShowImportDialog] = useState(false)
  const [selectedSDCard, setSelectedSDCard] = useState<SDCard | null>(null)
  const [isImporting, setIsImporting] = useState(false)
  const [importResult, setImportResult] = useState<CopyResult | null>(null)
  const [importId, setImportId] = useState<string | null>(null)
  const [isEditingDeadline, setIsEditingDeadline] = useState(false)
  const [homeDir, setHomeDir] = useState<string>('')
  const containerRef = useRef<HTMLDivElement>(null)
  const { sdCards, isScanning } = useSDCardScanner()

  const replaceHomeWithTilde = (path: string): string => {
    if (!homeDir) return path
    const normalizedHome = homeDir.replace(/\/$/, '')
    const normalizedPath = path.replace(/\/$/, '')
    return normalizedPath.startsWith(normalizedHome)
      ? normalizedPath.replace(normalizedHome, '~')
      : path
  }

  // IMPORTANT: DOM walk required for list↔detail transitions when containerRef switches elements
  // Simplifying to parent-only scroll breaks project creation flow. Change with caution.
  const scrollToTop = useCallback(() => {
    requestAnimationFrame(() => {
      let element: HTMLElement | null = containerRef.current
      while (element) {
        const style = window.getComputedStyle(element)
        if (style.overflowY === 'auto' || style.overflowY === 'scroll') {
          element.scrollTop = 0
          break
        }
        element = element.parentElement
      }
    })
  }, [])

  const loadProjects = useCallback(async () => {
    try {
      setLoading(true)
      const data = await invoke<Project[]>('list_projects')
      setProjects(data)
      return data
    } catch (err) {
      console.error('Failed to load projects:', err)
      return []
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    loadProjects()
    loadDestinations()
    loadArchiveLocation()
    loadHomeDirectory()
  }, [loadProjects])

  async function loadHomeDirectory() {
    try {
      const dir = await invoke<string>('get_home_directory')
      setHomeDir(dir)
    } catch (err) {
      console.error('Failed to load home directory:', err)
    }
  }

  // Handle initial project selection from navigation
  useEffect(() => {
    if (initialSelectedProjectId) {
      // Reload projects to get latest status when navigating to a project
      loadProjects().then((loadedProjects) => {
        // Find and select the project after reload
        const project = loadedProjects.find((p) => p.id === initialSelectedProjectId)
        if (project) {
          setSelectedProject(project)
        }
      })
    } else {
      // Clear selection when navigating to projects list
      setSelectedProject(null)
    }
  }, [initialSelectedProjectId, loadProjects])

  // Load import history when project is selected and scroll to top
  useEffect(() => {
    if (selectedProject) {
      loadProjectImportHistory(selectedProject.id)
      scrollToTop()
    }
  }, [selectedProject, scrollToTop])

  const handleBackToList = useCallback(() => {
    if (onBackFromProject) {
      onBackFromProject()
    } else {
      setSelectedProject(null)
    }
  }, [onBackFromProject])

  // ESC key to return to previous view
  useEffect(() => {
    if (!selectedProject) return

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        handleBackToList()
      }
    }

    window.addEventListener('keydown', handleKeyDown)
    return () => window.removeEventListener('keydown', handleKeyDown)
  }, [selectedProject, handleBackToList])

  async function loadDestinations() {
    try {
      const stored = localStorage.getItem('backup_destinations')
      if (stored) {
        setDestinations(JSON.parse(stored))
      }
    } catch (err) {
      console.error('Failed to load destinations:', err)
    }
  }

  async function loadArchiveLocation() {
    try {
      const stored = localStorage.getItem('archive_location')
      if (stored) {
        setArchiveLocation(stored)
      }
    } catch (err) {
      console.error('Failed to load archive location:', err)
    }
  }

  async function loadProjectImportHistory(projectId: string) {
    try {
      const history = await invoke<ImportHistory[]>('get_project_import_history', { projectId })
      setImportHistory(history)
    } catch (err) {
      console.error('Failed to load import history:', err)
    }
  }

  async function queueBackup(project: Project, destination: BackupDestination) {
    try {
      await invoke('queue_backup', {
        projectId: project.id,
        projectName: project.name,
        sourcePath: project.folderPath,
        destinationId: destination.id,
        destinationName: destination.name,
        destinationPath: destination.path,
      })
    } catch (err) {
      console.error('Failed to queue backup:', err)
    }
  }

  async function archiveProject(project: Project) {
    if (!archiveLocation) {
      alert('Please configure archive location in Settings')
      return
    }

    try {
      const job = await invoke<ArchiveJob>('create_archive', {
        projectId: project.id,
        projectName: project.name,
        sourcePath: project.folderPath,
        archiveLocation,
        compress: false,
        compressionFormat: null,
      })

      // Auto-start the archive job
      await invoke('start_archive', { jobId: job.id })

      setShowArchiveDialog(false)

      // Note: No need to reload - cache will be invalidated on backend when archive completes
    } catch (err) {
      console.error('Failed to archive project:', err)
      alert(`Failed to archive project: ${err}`)
    }
  }

  function handleProjectCreated(project: Project) {
    setShowCreateProject(false)
    setProjects([project, ...projects])
    setSelectedProject(project)
  }

  async function deleteProject() {
    if (!selectedProject) return

    setIsDeleting(true)

    try {
      await invoke('delete_project', { projectId: selectedProject.id })
      setShowDeleteDialog(false)
      setSelectedProject(null)
      await loadProjects()
      scrollToTop()
    } catch (err) {
      console.error('Failed to delete project:', err)
      alert(`Failed to delete project: ${err}`)
    } finally {
      setIsDeleting(false)
    }
  }

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

  async function handleOpenInApp(command: string, appName: string) {
    if (!selectedProject) return

    try {
      await invoke(command, { path: selectedProject.folderPath })

      // Auto-update status to Editing if not already Editing, Delivered, or Archived
      if (
        selectedProject.status !== ProjectStatus.Editing &&
        selectedProject.status !== ProjectStatus.Delivered &&
        selectedProject.status !== ProjectStatus.Archived
      ) {
        const updatedProject = await invoke<Project>('update_project_status', {
          projectId: selectedProject.id,
          newStatus: ProjectStatus.Editing,
        })
        setSelectedProject(updatedProject)
      }
    } catch (err) {
      alert(`Failed to open ${appName}: ${err}`)
    }
  }

  const createEmptyResult = (error?: string): CopyResult => ({
    success: false,
    error,
    filesCopied: 0,
    filesSkipped: 0,
    skippedFiles: [],
    totalBytes: 0,
    photosCopied: 0,
    videosCopied: 0,
  })

  async function handleStartImport() {
    if (!selectedProject || !selectedSDCard) return

    const currentImportId = `import-${Date.now()}-${Math.random().toString(36).substring(7)}`
    setImportId(currentImportId)
    setIsImporting(true)
    setImportResult(null)

    const startedAt = new Date().toISOString()

    try {
      await invoke('update_project_status', {
        projectId: selectedProject.id,
        newStatus: ProjectStatus.Importing,
      })
    } catch (err) {
      console.error('Failed to update project status:', err)
    }

    try {
      const sourcePaths = await invoke<string[]>('list_sd_card_files', {
        cardPath: selectedSDCard.path,
      })

      if (sourcePaths.length === 0) {
        const result = createEmptyResult('No photo or video files found on SD card')
        setImportResult(result)
        setIsImporting(false)
        setImportId(null)

        await invoke('save_import_history', {
          projectId: selectedProject.id,
          projectName: selectedProject.name,
          sourcePath: selectedSDCard.path,
          destinationPath: `${selectedProject.folderPath}/RAW`,
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

      const destination = `${selectedProject.folderPath}/RAW`

      const result = await invoke<CopyResult>('copy_files', {
        importId: currentImportId,
        sourcePaths,
        destination,
      })

      setImportResult(result)

      const wasCancelled = result.error?.includes('cancelled') ?? false

      if (!wasCancelled) {
        await invoke('save_import_history', {
          projectId: selectedProject.id,
          projectName: selectedProject.name,
          sourcePath: selectedSDCard.path,
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

      if (result.success && result.filesCopied > 0) {
        await loadProjectImportHistory(selectedProject.id)

        // Update project status to Editing after successful import
        try {
          const updatedProject = await invoke<Project>('update_project_status', {
            projectId: selectedProject.id,
            newStatus: ProjectStatus.Editing,
          })
          setSelectedProject(updatedProject)
        } catch (err) {
          console.error('Failed to update project status after import:', err)
        }
      }
    } catch (error) {
      console.error('Import failed:', error)
      setImportResult(createEmptyResult(String(error)))

      try {
        await invoke('save_import_history', {
          projectId: selectedProject.id,
          projectName: selectedProject.name,
          sourcePath: selectedSDCard.path,
          destinationPath: `${selectedProject.folderPath}/RAW`,
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

  async function handleCancelImport() {
    if (!importId) return

    try {
      await invoke('cancel_import', { importId })
    } catch (error) {
      console.error('Failed to cancel import:', error)
    }
  }

  async function handleDeadlineChange(newDeadline: string) {
    if (!selectedProject) return

    try {
      const updatedProject = await invoke<Project>('update_project_deadline', {
        projectId: selectedProject.id,
        deadline: newDeadline || null,
      })
      setSelectedProject(updatedProject)
      setIsEditingDeadline(false)
    } catch (error) {
      console.error('Failed to update deadline:', error)
      alert(`Failed to update deadline: ${error}`)
    }
  }

  if (loading) {
    return <div className="loading">Loading projects...</div>
  }

  if (selectedProject) {
    return (
      <div className="project-detail" ref={containerRef}>
        <div className="project-detail-header">
          <div>
            <button onClick={handleBackToList} className="btn-back">
              ← Back
            </button>
            <h1>{selectedProject.name}</h1>
          </div>
          <button
            onClick={() => setShowImportDialog(true)}
            className="btn btn-primary"
            title="Import from SD Card"
          >
            Import
          </button>
        </div>

        <div className="project-info">
          <div className="info-row">
            <span className="info-label">Client:</span>
            <span>{selectedProject.clientName}</span>
          </div>
          <div className="info-row">
            <span className="info-label">Date:</span>
            <span>{selectedProject.date}</span>
          </div>
          <div className="info-row">
            <span className="info-label">Type:</span>
            <span>{selectedProject.shootType}</span>
          </div>
          <div className="info-row">
            <span className="info-label">Status:</span>
            <span className={`project-status ${getStatusColor(selectedProject.status)}`}>
              {selectedProject.status}
            </span>
          </div>
          <div className="info-row">
            <span className="info-label">Deadline:</span>
            {isEditingDeadline ? (
              <DatePicker
                value={selectedProject.deadline || new Date().toISOString().split('T')[0]}
                onChange={handleDeadlineChange}
                autoOpen={true}
              />
            ) : (
              <span
                onClick={() => setIsEditingDeadline(true)}
                style={{ cursor: 'pointer', textDecoration: 'underline' }}
                title="Click to edit deadline"
              >
                {selectedProject.deadline ? formatDisplayDate(selectedProject.deadline) : 'Not set'}
              </span>
            )}
          </div>
          <div className="info-row">
            <span className="info-label">Location:</span>
            <span className="folder-path">{replaceHomeWithTilde(selectedProject.folderPath)}</span>
            <button
              onClick={(e) => {
                e.stopPropagation()
                invoke('reveal_in_finder', { path: selectedProject.folderPath })
              }}
              className="btn-icon"
              title="Show in Finder"
            >
              <img
                src={folderIcon}
                alt="Show in Finder"
                style={{ width: '30px', height: '30px' }}
              />
            </button>
          </div>
          {importHistory.length > 0 && importHistory[0].status === 'success' && (
            <>
              <div className="info-row">
                <span className="info-label">Photos Imported:</span>
                <span>{importHistory[0].photosCopied}</span>
              </div>
              <div className="info-row">
                <span className="info-label">Videos Imported:</span>
                <span>{importHistory[0].videosCopied}</span>
              </div>
            </>
          )}
        </div>

        <section className="project-actions">
          <h2>Actions</h2>

          <div className="backup-destinations">
            <h3>Open Photos In</h3>
            <p className="action-hint">Open this project's photos in your editing app</p>
            <div className="destination-list">
              <button
                onClick={() => handleOpenInApp('open_in_lightroom', 'Lightroom Classic')}
                className="destination-button"
              >
                <span className="destination-name">Lightroom Classic</span>
                <span className="destination-path">Photo editing application</span>
              </button>
              <button
                onClick={() => handleOpenInApp('open_in_aftershoot', 'AfterShoot')}
                className="destination-button"
              >
                <span className="destination-name">AfterShoot</span>
                <span className="destination-path">Photo culling & editing</span>
              </button>
            </div>
          </div>

          <div className="backup-destinations">
            <h3>Open Videos In</h3>
            <p className="action-hint">Open this project's videos in your editing app</p>
            <div className="destination-list">
              <button
                onClick={() => handleOpenInApp('open_in_davinci_resolve', 'DaVinci Resolve')}
                className="destination-button"
              >
                <span className="destination-name">DaVinci Resolve</span>
                <span className="destination-path">Video editing & color grading</span>
              </button>
            </div>
          </div>

          {destinations.length > 0 ? (
            <div className="backup-destinations">
              <h3>Backup to:</h3>
              <div className="destination-list">
                {destinations
                  .filter((d) => d.enabled)
                  .map((dest) => (
                    <button
                      key={dest.id}
                      onClick={() => queueBackup(selectedProject, dest)}
                      className="destination-button"
                    >
                      <span className="destination-name">{dest.name}</span>
                      <span className="destination-path">{dest.path}</span>
                    </button>
                  ))}
              </div>
            </div>
          ) : (
            <div className="empty-state">
              <p>No backup destinations configured</p>
              <p className="empty-state-hint">Add destinations in Settings to enable backup</p>
            </div>
          )}

          <div className="archive-action">
            <h3>Archive Project</h3>
            <p className="action-hint">
              Move this project to the archive location and update status to Archived
            </p>
            <button
              onClick={() => setShowArchiveDialog(true)}
              className="btn-archive"
              disabled={selectedProject.status === 'Archived'}
            >
              {selectedProject.status === 'Archived' ? 'Already Archived' : 'Archive Project'}
            </button>
          </div>

          <div className="delete-action">
            <h3>Delete Project</h3>
            <p className="action-hint">
              Permanently delete this project and all its files. This action cannot be undone.
            </p>
            <button onClick={() => setShowDeleteDialog(true)} className="btn-danger">
              Delete Project
            </button>
          </div>
        </section>

        {showArchiveDialog && (
          <div className="dialog-overlay" onClick={() => setShowArchiveDialog(false)}>
            <div className="dialog" onClick={(e) => e.stopPropagation()}>
              <h2>Archive Project</h2>
              <p>
                This will move the project to the archive location and update its status to
                Archived. The project will be removed from the active projects folder.
              </p>
              {archiveLocation ? (
                <div className="archive-info">
                  <p>
                    <strong>Archive Location:</strong>
                  </p>
                  <p className="folder-path">{archiveLocation}</p>
                </div>
              ) : (
                <div className="warning">
                  <p>No archive location configured. Please set one in Settings first.</p>
                </div>
              )}
              <div className="dialog-actions">
                <button onClick={() => setShowArchiveDialog(false)} className="btn-secondary">
                  Cancel
                </button>
                <button
                  onClick={() => archiveProject(selectedProject)}
                  className="btn-primary"
                  disabled={!archiveLocation}
                >
                  Confirm Archive
                </button>
              </div>
            </div>
          </div>
        )}

        {showDeleteDialog && (
          <div className="dialog-overlay" onClick={() => !isDeleting && setShowDeleteDialog(false)}>
            <div className="dialog" onClick={(e) => e.stopPropagation()}>
              {isDeleting ? (
                <>
                  <h2>Deleting Project</h2>
                  <div
                    className="flex flex-col gap-md"
                    style={{ alignItems: 'center', padding: 'var(--space-lg)' }}
                  >
                    <div className="spinner"></div>
                    <p className="text-secondary">Hang tight, removing your project...</p>
                  </div>
                </>
              ) : (
                <>
                  <h2>Delete Project</h2>
                  <p>
                    Are you sure you want to delete <strong>{selectedProject.name}</strong>? This
                    will permanently delete the project folder and all its contents.
                  </p>
                  <p className="warning">This action cannot be undone.</p>
                  <div className="dialog-actions">
                    <button onClick={() => setShowDeleteDialog(false)} className="btn-secondary">
                      Cancel
                    </button>
                    <button onClick={deleteProject} className="btn-danger">
                      Delete Project
                    </button>
                  </div>
                </>
              )}
            </div>
          </div>
        )}

        {showImportDialog && (
          <div
            className="dialog-overlay"
            onClick={() => !isImporting && setShowImportDialog(false)}
          >
            <div className="dialog" onClick={(e) => e.stopPropagation()}>
              {!isImporting && !importResult ? (
                <>
                  <h2>Import from SD Card</h2>
                  <p className="text-secondary">Select an SD card to import files from</p>

                  {isScanning && sdCards.length === 0 ? (
                    <div className="flex flex-col gap-md" style={{ padding: 'var(--space-lg)' }}>
                      <p className="text-secondary">Scanning for SD cards...</p>
                    </div>
                  ) : sdCards.length === 0 ? (
                    <div className="flex flex-col gap-md" style={{ padding: 'var(--space-lg)' }}>
                      <p className="text-secondary">
                        No SD cards detected. Insert an SD card to continue.
                      </p>
                    </div>
                  ) : (
                    <div className="flex flex-col gap-sm" style={{ marginTop: 'var(--space-md)' }}>
                      {sdCards.map((card) => (
                        <div
                          key={card.path}
                          className={`destination-button ${selectedSDCard?.path === card.path ? 'selected' : ''}`}
                          onClick={() => setSelectedSDCard(card)}
                          style={{ cursor: 'pointer' }}
                        >
                          <span className="destination-name">{card.name}</span>
                          <span className="destination-path">
                            {card.fileCount} files · {(card.size / 1024 / 1024 / 1024).toFixed(1)}{' '}
                            GB
                          </span>
                        </div>
                      ))}
                    </div>
                  )}

                  <div className="dialog-actions">
                    <button
                      onClick={() => {
                        setShowImportDialog(false)
                        setSelectedSDCard(null)
                      }}
                      className="btn-secondary"
                    >
                      Cancel
                    </button>
                    <button
                      onClick={handleStartImport}
                      className="btn-primary"
                      disabled={!selectedSDCard}
                    >
                      Start Import
                    </button>
                  </div>
                </>
              ) : isImporting ? (
                <>
                  <h2>Importing Files</h2>
                  <div
                    className="flex flex-col gap-md"
                    style={{ alignItems: 'center', padding: 'var(--space-lg)' }}
                  >
                    <div className="spinner"></div>
                    <p className="text-secondary">Importing from {selectedSDCard?.name}...</p>
                  </div>
                  <div className="dialog-actions">
                    <button onClick={handleCancelImport} className="btn-secondary">
                      Cancel Import
                    </button>
                  </div>
                </>
              ) : importResult ? (
                <>
                  <h2>Import Complete</h2>
                  <div className="flex flex-col gap-sm" style={{ padding: 'var(--space-md)' }}>
                    {importResult.success ? (
                      <>
                        <p className="text-success">
                          Successfully imported {importResult.filesCopied} files
                        </p>
                        {importResult.photosCopied > 0 && (
                          <p>Photos: {importResult.photosCopied}</p>
                        )}
                        {importResult.videosCopied > 0 && (
                          <p>Videos: {importResult.videosCopied}</p>
                        )}
                        {importResult.filesSkipped > 0 && (
                          <p className="text-warning">Files skipped: {importResult.filesSkipped}</p>
                        )}
                      </>
                    ) : (
                      <>
                        <p className="text-error">Import failed</p>
                        {importResult.error && (
                          <p className="text-secondary">{importResult.error}</p>
                        )}
                      </>
                    )}
                  </div>
                  <div className="dialog-actions">
                    <button
                      onClick={() => {
                        setShowImportDialog(false)
                        setSelectedSDCard(null)
                        setImportResult(null)
                      }}
                      className="btn-primary"
                    >
                      Done
                    </button>
                  </div>
                </>
              ) : null}
            </div>
          </div>
        )}
      </div>
    )
  }

  return (
    <>
      <div className="projects" ref={containerRef}>
        <div className="projects-header">
          <div className="flex flex-between">
            <h1>Projects</h1>
            <button className="btn btn-primary" onClick={() => setShowCreateProject(true)}>
              Create Project
            </button>
          </div>
        </div>

        {projects.length === 0 ? (
          <div className="empty-state">
            <p>No projects yet</p>
            <p className="empty-state-hint">Click "Create Project" to get started</p>
          </div>
        ) : (
          <div className="projects-list">
            {projects.map((project) => (
              <div
                key={project.id}
                className="project-card"
                onClick={() => setSelectedProject(project)}
              >
                <div className="project-card-header">
                  <h3>{project.name}</h3>
                  <span className={`project-status ${getStatusColor(project.status)}`}>
                    {project.status}
                  </span>
                </div>

                <div className="project-card-info">
                  <div className="info-item">
                    <span className="info-label">Client:</span>
                    <span>{project.clientName}</span>
                  </div>
                  <div className="info-item">
                    <span className="info-label">Date:</span>
                    <span>{project.date}</span>
                  </div>
                  <div className="info-item">
                    <span className="info-label">Type:</span>
                    <span>{project.shootType}</span>
                  </div>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      {showCreateProject && (
        <div className="dialog-overlay" onClick={() => setShowCreateProject(false)}>
          <div className="dialog" onClick={(e) => e.stopPropagation()}>
            <h2>Create New Project</h2>
            <CreateProject
              onProjectCreated={handleProjectCreated}
              onCancel={() => setShowCreateProject(false)}
            />
          </div>
        </div>
      )}
    </>
  )
}
