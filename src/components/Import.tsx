import { useEffect, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import type { SDCard, Project, CopyResult } from '../types'
import { CreateProject } from './CreateProject'

interface ImportProps {
  sdCards: SDCard[]
  isScanning: boolean
  onRefresh: () => void
  onImportComplete: (projectId: string) => void
}

export function Import({ sdCards, isScanning, onRefresh, onImportComplete }: ImportProps) {
  const [activeCardPath, setActiveCardPath] = useState<string | null>(null)

  return (
    <>
      <div className="content-header">
        <div className="flex" style={{ justifyContent: 'space-between', alignItems: 'center' }}>
          <div>
            <h1>Import from SD Card</h1>
            <p className="text-secondary">Detect and import files from SD cards</p>
          </div>
          <button className="btn btn-primary" onClick={onRefresh} disabled={isScanning}>
            {isScanning ? 'Scanning...' : 'Refresh'}
          </button>
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
  const [cancelRequested, setCancelRequested] = useState(false)

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

    setIsImporting(true)
    setImportResult(null)
    setCancelRequested(false)

    const startedAt = new Date().toISOString()

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
        }
        setImportResult(result)
        setIsImporting(false)

        // Save to history
        await invoke('save_import_history', {
          projectId: project.id,
          projectName: project.name,
          sourcePath: card.path,
          destinationPath: `${project.folderPath}/RAW`,
          filesCopied: 0,
          filesSkipped: 0,
          totalBytes: 0,
          startedAt,
          errorMessage: result.error,
        })

        return
      }

      const destination = `${project.folderPath}/RAW`

      const result = await invoke<CopyResult>('copy_files', {
        sourcePaths,
        destination,
      })

      setImportResult(result)

      // Save to history
      await invoke('save_import_history', {
        projectId: project.id,
        projectName: project.name,
        sourcePath: card.path,
        destinationPath: destination,
        filesCopied: result.filesCopied,
        filesSkipped: result.filesSkipped,
        totalBytes: 0,
        startedAt,
        errorMessage: result.error || null,
      })

      // Navigate to project view on successful import
      if (result.success && result.filesCopied > 0) {
        setTimeout(() => onImportComplete(project.id), 1500)
      }
    } catch (error) {
      console.error('Import failed:', error)
      setImportResult({
        success: false,
        error: String(error),
        filesCopied: 0,
        filesSkipped: 0,
        skippedFiles: [],
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
          startedAt,
          errorMessage: error as string,
        })
      } catch (historyError) {
        console.error('Failed to save import history:', historyError)
      }
    } finally {
      setIsImporting(false)
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

  const handleCancelImport = () => {
    setCancelRequested(true)
  }

  if (isImporting) {
    return (
      <div className="card">
        <div className="flex flex-col gap-md">
          <div>
            <h3>{card.name}</h3>
            <p className="text-secondary text-sm">
              {cancelRequested ? 'Import continuing in background...' : 'Importing files...'}
            </p>
          </div>
          {!cancelRequested && (
            <button className="btn" onClick={handleCancelImport}>
              Stop Waiting
            </button>
          )}
        </div>
      </div>
    )
  }

  if (importResult) {
    return (
      <div className="card">
        <div className="flex flex-col gap-md">
          <div>
            <h3>{card.name}</h3>
            <p className={`text-sm ${importResult.success ? 'text-success' : 'text-error'}`}>
              {importResult.success ? 'Import completed' : 'Import failed'}
            </p>
          </div>

          <div className="text-sm">
            <p>Files copied: {importResult.filesCopied}</p>
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
