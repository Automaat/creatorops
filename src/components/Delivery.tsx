import { useEffect, useRef, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { formatBytes } from '../utils/formatting'
import { sortProjectsByStatus } from '../utils/project'
import type {
  DeliveryDestination,
  DeliveryJob,
  DeliveryProgress,
  Project,
  ProjectFile,
} from '../types'

export function Delivery() {
  const [projects, setProjects] = useState<Project[]>([])
  const [selectedProject, setSelectedProject] = useState<Project | null>()
  const [projectFiles, setProjectFiles] = useState<ProjectFile[]>([])
  const [selectedFiles, setSelectedFiles] = useState<Set<string>>(new Set())
  const [destinations, setDestinations] = useState<DeliveryDestination[]>([])
  const [deliveryJobs, setDeliveryJobs] = useState<DeliveryJob[]>([])
  const [namingTemplate, setNamingTemplate] = useState('')
  const [loading, setLoading] = useState(true)
  const [showProjectSelect, setShowProjectSelect] = useState(false)
  const [dropdownPosition, setDropdownPosition] = useState<{
    top: number
    left: number
    width: number
  } | null>()
  const triggerRef = useRef<HTMLButtonElement>(null)
  const dropdownRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    void loadProjects()
    void loadDestinations()
    void loadDeliveryQueue()

    // Listen for delivery progress events
    const unlisten = listen<DeliveryProgress>('delivery-progress', (event) => {
      const progress = event.payload
      setDeliveryJobs((prev) =>
        prev.map((job) =>
          job.id === progress.jobId
            ? {
                ...job,
                bytesTransferred: progress.bytesTransferred,
                filesCopied: progress.currentFile,
              }
            : job
        )
      )
    })

    return () => {
      void unlisten.then((fn) => fn()).catch(() => {})
    }
  }, [])

  useEffect(() => {
    if (selectedProject) {
      void loadProjectFiles(selectedProject.id)
    }
  }, [selectedProject])

  useEffect(() => {
    if (showProjectSelect) {
      updateDropdownPosition()
    }
  }, [showProjectSelect])

  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (
        showProjectSelect &&
        triggerRef.current &&
        dropdownRef.current &&
        event.target instanceof Node &&
        !triggerRef.current.contains(event.target) &&
        !dropdownRef.current.contains(event.target)
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
        left: rect.left,
        top: rect.bottom + 8,
        width: rect.width,
      })
    }
  }

  async function loadProjects() {
    try {
      setLoading(true)
      const data = await invoke<Project[]>('list_projects')
      const sortedProjects = sortProjectsByStatus(data)
      setProjects(sortedProjects)
    } catch (error) {
      console.error('Failed to load projects:', error)
    } finally {
      setLoading(false)
    }
  }

  async function loadProjectFiles(projectId: string) {
    try {
      const files = await invoke<ProjectFile[]>('list_project_files', { projectId })
      setProjectFiles(files)
      setSelectedFiles(new Set())
    } catch (error) {
      console.error('Failed to load project files:', error)
    }
  }

  async function loadDestinations() {
    try {
      const stored = localStorage.getItem('delivery_destinations')
      if (stored) {
        const parsed: unknown = JSON.parse(stored)
        if (Array.isArray(parsed)) {
          setDestinations(parsed as DeliveryDestination[])
        }
      }
    } catch (error) {
      console.error('Failed to load destinations:', error)
    }
  }

  async function loadDeliveryQueue() {
    try {
      const queue = await invoke<DeliveryJob[]>('get_delivery_queue')
      setDeliveryJobs(queue)
    } catch (error) {
      console.error('Failed to load delivery queue:', error)
    }
  }

  function toggleFileSelection(filePath: string) {
    setSelectedFiles((prev) => {
      const next = new Set(prev)
      if (next.has(filePath)) {
        next.delete(filePath)
      } else {
        next.add(filePath)
      }
      return next
    })
  }

  function selectAll() {
    setSelectedFiles(new Set(projectFiles.map((f) => f.path)))
  }

  function deselectAll() {
    setSelectedFiles(new Set())
  }

  async function createDelivery(destination: DeliveryDestination) {
    if (!selectedProject || selectedFiles.size === 0) {
      return
    }

    try {
      const job = await invoke<DeliveryJob>('create_delivery', {
        deliveryPath: destination.path,
        namingTemplate: namingTemplate || undefined,
        projectId: selectedProject.id,
        projectName: selectedProject.name,
        selectedFiles: [...selectedFiles],
      })

      setDeliveryJobs((prev) => [...prev, job])

      // Auto-start the delivery
      await invoke('start_delivery', { jobId: job.id })
    } catch (error) {
      console.error('Failed to create delivery:', error)
    }
  }

  async function removeJob(jobId: string) {
    try {
      await invoke('remove_delivery_job', { jobId })
      setDeliveryJobs((prev) => prev.filter((j) => j.id !== jobId))
    } catch (error) {
      console.error('Failed to remove job:', error)
    }
  }

  function getStatusClass(status: string): string {
    switch (status) {
      case 'pending': {
        return 'status-pending'
      }
      case 'inprogress': {
        return 'status-inprogress'
      }
      case 'completed': {
        return 'status-completed'
      }
      case 'failed': {
        return 'status-failed'
      }
      default: {
        return ''
      }
    }
  }

  function getProjectStatusColor(status: string): string {
    switch (status) {
      case 'New': {
        return 'status-new'
      }
      case 'Importing': {
        return 'status-importing'
      }
      case 'Editing': {
        return 'status-editing'
      }
      case 'Delivered': {
        return 'status-delivered'
      }
      case 'Archived': {
        return 'status-archived'
      }
      default: {
        return ''
      }
    }
  }

  if (loading) {
    return <div className="loading">Loading...</div>
  }

  return (
    <div className="delivery">
      <div className="delivery-header">
        <h1>Client Delivery</h1>
      </div>

      <div className="delivery-content">
        <section className="delivery-section">
          <h2>1. Select Project</h2>
          <div className="project-dropdown-container">
            <button
              ref={triggerRef}
              className="project-dropdown-trigger"
              onClick={() => setShowProjectSelect(!showProjectSelect)}
            >
              {selectedProject ? (
                <div className="project-dropdown-selected">
                  <div className="project-select-header">
                    <h4>{selectedProject.name}</h4>
                    <span
                      className={`project-status ${getProjectStatusColor(selectedProject.status)}`}
                    >
                      {selectedProject.status}
                    </span>
                  </div>
                  <div className="project-select-info">
                    <span className="text-secondary text-sm">
                      {selectedProject.clientName} · {selectedProject.date} ·{' '}
                      {selectedProject.shootType}
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
                  left: `${dropdownPosition.left}px`,
                  top: `${dropdownPosition.top}px`,
                  width: `${dropdownPosition.width}px`,
                }}
              >
                {projects.length > 0 ? (
                  projects.map((project) => (
                    <div
                      key={project.id}
                      className={`project-select-card ${selectedProject?.id === project.id ? 'selected' : ''}`}
                      onClick={() => {
                        setSelectedProject(project)
                        setShowProjectSelect(false)
                      }}
                    >
                      <div className="project-select-header">
                        <h4>{project.name}</h4>
                        <span className={`project-status ${getProjectStatusColor(project.status)}`}>
                          {project.status}
                        </span>
                      </div>
                      <div className="project-select-info">
                        <span className="text-secondary text-sm">
                          {project.clientName} · {project.date} · {project.shootType}
                        </span>
                      </div>
                    </div>
                  ))
                ) : (
                  <div className="empty-state">
                    <p className="text-secondary">No projects available</p>
                  </div>
                )}
              </div>
            )}
          </div>
        </section>

        {selectedProject && (
          <section className="delivery-section">
            <div className="section-header">
              <h2>2. Select Files</h2>
              <div className="file-actions">
                <button onClick={selectAll} className="btn-secondary">
                  Select All
                </button>
                <button onClick={deselectAll} className="btn-secondary">
                  Deselect All
                </button>
              </div>
            </div>

            <div className="file-list">
              {projectFiles.length === 0 ? (
                <div className="empty-state">No files found in project</div>
              ) : (
                projectFiles.map((file) => (
                  <div
                    key={file.path}
                    className={`file-item ${selectedFiles.has(file.path) ? 'selected' : ''}`}
                    onClick={() => toggleFileSelection(file.path)}
                  >
                    <input
                      type="checkbox"
                      checked={selectedFiles.has(file.path)}
                      onChange={() => toggleFileSelection(file.path)}
                    />
                    <div className="file-info">
                      <span className="file-name">{file.name}</span>
                      <span className="file-meta">
                        {file.relativePath} • {formatBytes(file.size)}
                      </span>
                    </div>
                  </div>
                ))
              )}
            </div>

            <div className="selected-count">
              {selectedFiles.size} file{selectedFiles.size !== 1 ? 's' : ''} selected
            </div>
          </section>
        )}

        {selectedProject && selectedFiles.size > 0 && (
          <section className="delivery-section">
            <h2>3. Naming Template (Optional)</h2>
            <input
              type="text"
              value={namingTemplate}
              onChange={(e) => setNamingTemplate(e.target.value)}
              placeholder="e.g., {name}_{index}.{ext}"
              className="naming-template-input"
            />
            <p className="hint">
              Use {'{index}'} for file number, {'{name}'} for original name, {'{ext}'} for extension
            </p>
          </section>
        )}

        {selectedProject && selectedFiles.size > 0 && (
          <section className="delivery-section">
            <h2>4. Choose Destination</h2>
            {destinations.filter((d) => d.enabled).length === 0 ? (
              <div className="empty-state">
                <p>No delivery destinations configured</p>
                <p className="empty-state-hint">Add destinations in Settings</p>
              </div>
            ) : (
              <div className="destination-list">
                {destinations
                  .filter((d) => d.enabled)
                  .map((dest) => (
                    <button
                      key={dest.id}
                      onClick={() => void createDelivery(dest)}
                      className="destination-button"
                    >
                      <span className="destination-name">{dest.name}</span>
                      <span className="destination-path">{dest.path}</span>
                    </button>
                  ))}
              </div>
            )}
          </section>
        )}

        <section className="delivery-section">
          <h2>Delivery Queue</h2>
          {deliveryJobs.length === 0 ? (
            <div className="empty-state">No deliveries yet</div>
          ) : (
            <div className="job-list">
              {deliveryJobs.map((job) => (
                <div key={job.id} className="job-card">
                  <div className="job-header">
                    <div className="job-info">
                      <h3>{job.projectName}</h3>
                      <span className={`job-status ${getStatusClass(job.status)}`}>
                        {job.status}
                      </span>
                    </div>
                    {(job.status === 'completed' || job.status === 'failed') && (
                      <button onClick={() => void removeJob(job.id)} className="btn-remove">
                        Remove
                      </button>
                    )}
                  </div>

                  <div className="job-details">
                    <div className="detail-item">
                      <span className="detail-label">Destination:</span>
                      <span>{job.deliveryPath}</span>
                    </div>
                    <div className="detail-item">
                      <span className="detail-label">Files:</span>
                      <span>
                        {job.filesCopied} / {job.totalFiles}
                      </span>
                    </div>
                    <div className="detail-item">
                      <span className="detail-label">Size:</span>
                      <span>
                        {formatBytes(job.bytesTransferred)} / {formatBytes(job.totalBytes)}
                      </span>
                    </div>
                  </div>

                  {job.status === 'inprogress' && (
                    <div className="progress-bar">
                      <div
                        className="progress-fill"
                        style={{
                          width: `${(job.bytesTransferred / job.totalBytes) * 100}%`,
                        }}
                      />
                    </div>
                  )}

                  {job.errorMessage && <div className="error-message">{job.errorMessage}</div>}

                  {job.manifestPath && (
                    <div className="manifest-link">
                      <span>Manifest: {job.manifestPath}</span>
                    </div>
                  )}
                </div>
              ))}
            </div>
          )}
        </section>
      </div>
    </div>
  )
}
