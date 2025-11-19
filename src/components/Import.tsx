import { useState, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type { SDCard, Project, ImportProgress, CopyResult } from '../types'
import { CreateProject } from './CreateProject'

interface ImportProps {
  sdCards: SDCard[]
  isScanning: boolean
  onRefresh: () => void
}

export function Import({ sdCards, isScanning, onRefresh }: ImportProps) {
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
                <SDCardItem key={card.path} card={card} />
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
}

function SDCardItem({ card }: SDCardItemProps) {
  const [showProjectSelect, setShowProjectSelect] = useState(false)
  const [projects, setProjects] = useState<Project[]>([])
  const [selectedProject, setSelectedProject] = useState<string>('')
  const [showCreateNew, setShowCreateNew] = useState(false)
  const [isImporting, setIsImporting] = useState(false)
  const [importProgress, setImportProgress] = useState<ImportProgress | null>(null)
  const [importResult, setImportResult] = useState<CopyResult | null>(null)

  const usedSpace = card.size - card.freeSpace
  const usedPercent = card.size > 0 ? (usedSpace / card.size) * 100 : 0

  useEffect(() => {
    if (showProjectSelect) {
      loadProjects()
    }
  }, [showProjectSelect])

  useEffect(() => {
    if (!isImporting) return

    const unlisten = listen<ImportProgress>('import-progress', (event) => {
      setImportProgress(event.payload)
    })

    return () => {
      unlisten.then((fn) => fn())
    }
  }, [isImporting])

  const loadProjects = async () => {
    try {
      const projectList = await invoke<Project[]>('list_projects')
      setProjects(projectList)
    } catch (error) {
      console.error('Failed to load projects:', error)
    }
  }

  const handleImportClick = () => {
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
    setImportProgress(null)
    setImportResult(null)

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

      // Calculate total bytes transferred (approximation based on progress)
      const totalBytes = importProgress?.totalBytes || 0

      // Save to history
      await invoke('save_import_history', {
        projectId: project.id,
        projectName: project.name,
        sourcePath: card.path,
        destinationPath: destination,
        filesCopied: result.filesCopied,
        filesSkipped: result.filesSkipped,
        totalBytes,
        startedAt,
        errorMessage: result.error || null,
      })
    } catch (error) {
      console.error('Import failed:', error)
      const result = {
        success: false,
        error: error as string,
        filesCopied: 0,
        filesSkipped: 0,
        skippedFiles: [],
      }
      setImportResult(result)

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
      setImportProgress(null)
    }
  }

  if (showCreateNew) {
    return (
      <div className="card">
        <h3 className="mb-md">Create New Project for {card.name}</h3>
        <CreateProject
          onProjectCreated={handleProjectCreated}
          onCancel={() => setShowCreateNew(false)}
        />
      </div>
    )
  }

  if (showProjectSelect && !isImporting && !importResult) {
    return (
      <div className="card">
        <div className="flex flex-col gap-md">
          <div>
            <h3>{card.name}</h3>
            <p className="text-secondary text-sm">{card.path}</p>
            <p className="text-sm mt-xs">
              {card.fileCount} files · {formatBytes(usedSpace)}
            </p>
          </div>

          <div>
            <label htmlFor={`project-select-${card.path}`} className="text-sm font-medium mb-xs">
              Select Project
            </label>
            <select
              id={`project-select-${card.path}`}
              className="input w-full"
              value={selectedProject}
              onChange={(e) => handleProjectSelect(e.target.value)}
            >
              <option value="">Choose a project...</option>
              <option value="__new__">+ Create New Project</option>
              {projects.map((project) => (
                <option key={project.id} value={project.id}>
                  {project.name} - {project.clientName} ({project.date})
                </option>
              ))}
            </select>
          </div>

          <div className="flex gap-sm">
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
    )
  }

  if (isImporting || importProgress) {
    return (
      <div className="card">
        <div className="flex flex-col gap-md">
          <div>
            <h3>{card.name}</h3>
            <p className="text-secondary text-sm">Importing...</p>
          </div>

          {importProgress && (
            <>
              <div>
                <div
                  className="flex"
                  style={{ justifyContent: 'space-between', marginBottom: 'var(--space-xs)' }}
                >
                  <span className="text-sm">
                    {importProgress.currentFile} / {importProgress.totalFiles} files
                  </span>
                  <span className="text-sm">
                    {formatBytes(importProgress.bytesTransferred)} /{' '}
                    {formatBytes(importProgress.totalBytes)}
                  </span>
                </div>
                <div className="progress">
                  <div
                    className="progress-bar"
                    style={{
                      width: `${(importProgress.bytesTransferred / importProgress.totalBytes) * 100}%`,
                    }}
                  />
                </div>
              </div>

              <div className="text-sm text-secondary">
                <p>Current: {importProgress.fileName}</p>
                <p>
                  Speed: {formatBytes(importProgress.speed)}/s · ETA:{' '}
                  {formatTime(importProgress.eta)}
                </p>
              </div>
            </>
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

  return (
    <div className="card">
      <div className="flex flex-col gap-md">
        <div className="flex" style={{ justifyContent: 'space-between', alignItems: 'center' }}>
          <div>
            <h3>{card.name}</h3>
            <p className="text-secondary text-sm">{card.path}</p>
            <p className="text-secondary text-xs mt-xxs">
              {card.deviceType}
              {card.isRemovable && ' • Removable'}
            </p>
          </div>
          <button className="btn btn-primary" onClick={handleImportClick}>
            Import
          </button>
        </div>

        <div>
          <div
            className="flex"
            style={{ justifyContent: 'space-between', marginBottom: 'var(--space-xs)' }}
          >
            <span className="text-sm text-secondary">{card.fileCount} files</span>
            <span className="text-sm text-secondary">
              {formatBytes(usedSpace)} / {formatBytes(card.size)}
            </span>
          </div>
          <div className="progress">
            <div className="progress-bar" style={{ width: `${usedPercent}%` }} />
          </div>
        </div>
      </div>
    </div>
  )
}

function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes < 0) return '0 B'
  if (bytes === 0) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return `${(bytes / Math.pow(k, i)).toFixed(2)} ${sizes[i]}`
}

function formatTime(seconds: number): string {
  if (seconds < 60) return `${Math.round(seconds)}s`
  const minutes = Math.floor(seconds / 60)
  const secs = Math.round(seconds % 60)
  if (minutes < 60) return `${minutes}m ${secs}s`
  const hours = Math.floor(minutes / 60)
  const mins = minutes % 60
  return `${hours}h ${mins}m`
}
