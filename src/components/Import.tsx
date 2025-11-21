import { useEffect, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import type { SDCard, Project, CopyResult } from '../types'
import { CreateProject } from './CreateProject'

const POST_IMPORT_DELAY_MS = 1500 // Allow user to see success message

interface ImportProps {
  sdCards: SDCard[]
  isScanning: boolean
  onImportComplete: (projectId: string) => void
}

export function Import({ sdCards, isScanning, onImportComplete }: ImportProps) {
  const [activeCardPath, setActiveCardPath] = useState<string | null>(null)

  // Reset active card when the active card is no longer in the list
  useEffect(() => {
    if (activeCardPath && !sdCards.some((card) => card.path === activeCardPath)) {
      setActiveCardPath(null)
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [sdCards.length, activeCardPath])

  return (
    <>
      <div className="content-header">
        <div>
          <h1>Import from SD Card</h1>
          <p className="text-secondary">Detect and import files from SD cards</p>
        </div>
      </div>
      <div className="content-body">
        <div className="flex flex-col gap-lg">
          {sdCards.length === 0 ? (
            <div className="card">
              <p className="text-secondary">
                {isScanning
                  ? 'Scanning for SD cards...'
                  : 'No SD cards detected. Insert an SD card and click Refresh.'}
              </p>
            </div>
          ) : (
            <div className="flex flex-col gap-md">
              {sdCards.map((card) => (
                <SDCardItem
                  key={card.path}
                  card={card}
                  onImportComplete={onImportComplete}
                  isActive={activeCardPath === card.path}
                  onActivate={() => setActiveCardPath(card.path)}
                />
              ))}
            </div>
          )}
        </div>
      </div>
    </>
  )
}

interface SDCardItemProps {
  card: SDCard
  onImportComplete: (projectId: string) => void
  isActive: boolean
  onActivate: () => void
}

function SDCardItem({ card, onImportComplete, isActive, onActivate }: SDCardItemProps) {
  const [showProjectSelect, setShowProjectSelect] = useState(false)
  const [projects, setProjects] = useState<Project[]>([])
  const [selectedProject, setSelectedProject] = useState<string>('')
  const [showCreateNew, setShowCreateNew] = useState(false)
  const [isImporting, setIsImporting] = useState(false)
  const [importResult, setImportResult] = useState<CopyResult | null>(null)
  const [importId, setImportId] = useState<string | null>(null)

  useEffect(() => {
    if (showProjectSelect) {
      loadProjects()
    }
  }, [showProjectSelect])

  const loadProjects = async () => {
    try {
      const projectList = await invoke<Project[]>('list_projects')
      setProjects(projectList)
    } catch (error) {
      console.error('Failed to load projects:', error)
    }
  }

  const handleImportClick = () => {
    onActivate()
    setShowProjectSelect(true)
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
    setProjects([project, ...projects])
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

    const startedAt = new Date().toISOString()

    // Update project status to Importing
    try {
      await invoke('update_project_status', {
        projectId: project.id,
        newStatus: 'Importing',
      })
    } catch (err) {
      console.error('Failed to update project status:', err)
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

  // Only show expanded view if this card is active
  if (!isActive) {
    return (
      <div className="card" onClick={handleImportClick} style={{ cursor: 'pointer' }}>
        <div className="flex flex-col gap-md">
          <div className="flex" style={{ justifyContent: 'space-between', alignItems: 'center' }}>
            <div>
              <h3>{card.name}</h3>
            </div>
            <button
              className="btn btn-primary"
              onClick={(e) => {
                e.stopPropagation()
                handleImportClick()
              }}
            >
              Import
            </button>
          </div>
        </div>
      </div>
    )
  }

  if (showProjectSelect && !isImporting && !importResult) {
    return (
      <>
        <div className="card">
          <div className="flex flex-col gap-md">
            <div>
              <h3>{card.name}</h3>
            </div>

            <div className="flex flex-col gap-xs">
              <label htmlFor={`project-select-${card.path}`} className="text-sm font-medium">
                Select Project
              </label>
              <select
                id={`project-select-${card.path}`}
                className="project-select"
                value={selectedProject}
                onChange={(e) => handleProjectSelect(e.target.value)}
              >
                <option value="">Choose a project...</option>
                {projects.map((project) => (
                  <option key={project.id} value={project.id}>
                    {project.name} - {project.clientName} ({project.date})
                  </option>
                ))}
                <option value="__new__">+ Create New Project</option>
              </select>
            </div>

            <div className="flex gap-sm" style={{ marginTop: 'var(--space-sm)' }}>
              <button
                className="btn btn-primary"
                onClick={handleStartImport}
                disabled={!selectedProject}
              >
                Start Import
              </button>
              <button className="btn" onClick={() => setShowProjectSelect(false)}>
                Cancel
              </button>
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
    return (
      <div className="card">
        <div className="flex flex-col gap-md">
          <div>
            <h3>{card.name}</h3>
            <p className="text-secondary text-sm">Importing files...</p>
          </div>
          <button className="btn" onClick={handleCancelImport}>
            Cancel Import
          </button>
        </div>
      </div>
    )
  }

  if (importResult) {
    const wasCancelled = importResult.error?.includes('cancelled') || false

    return (
      <div className="card">
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
    )
  }

  // This should never be reached, but just in case return null
  return null
}
