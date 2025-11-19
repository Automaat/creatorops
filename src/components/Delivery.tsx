import { useEffect, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type {
  Project,
  ProjectFile,
  DeliveryJob,
  DeliveryDestination,
  DeliveryProgress,
} from '../types'

export function Delivery() {
  const [projects, setProjects] = useState<Project[]>([])
  const [selectedProject, setSelectedProject] = useState<Project | null>(null)
  const [projectFiles, setProjectFiles] = useState<ProjectFile[]>([])
  const [selectedFiles, setSelectedFiles] = useState<Set<string>>(new Set())
  const [destinations, setDestinations] = useState<DeliveryDestination[]>([])
  const [deliveryJobs, setDeliveryJobs] = useState<DeliveryJob[]>([])
  const [namingTemplate, setNamingTemplate] = useState('')
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    loadProjects()
    loadDestinations()
    loadDeliveryQueue()

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
      unlisten.then((fn) => fn())
    }
  }, [])

  useEffect(() => {
    if (selectedProject) {
      loadProjectFiles(selectedProject.id)
    }
  }, [selectedProject])

  async function loadProjects() {
    try {
      setLoading(true)
      const data = await invoke<Project[]>('list_projects')
      setProjects(data)
    } catch (err) {
      console.error('Failed to load projects:', err)
    } finally {
      setLoading(false)
    }
  }

  async function loadProjectFiles(projectId: string) {
    try {
      const files = await invoke<ProjectFile[]>('list_project_files', { projectId })
      setProjectFiles(files)
      setSelectedFiles(new Set())
    } catch (err) {
      console.error('Failed to load project files:', err)
    }
  }

  async function loadDestinations() {
    try {
      const stored = localStorage.getItem('delivery_destinations')
      if (stored) {
        setDestinations(JSON.parse(stored))
      }
    } catch (err) {
      console.error('Failed to load destinations:', err)
    }
  }

  async function loadDeliveryQueue() {
    try {
      const queue = await invoke<DeliveryJob[]>('get_delivery_queue')
      setDeliveryJobs(queue)
    } catch (err) {
      console.error('Failed to load delivery queue:', err)
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
    if (!selectedProject || selectedFiles.size === 0) return

    try {
      const job = await invoke<DeliveryJob>('create_delivery', {
        projectId: selectedProject.id,
        projectName: selectedProject.name,
        selectedFiles: Array.from(selectedFiles),
        deliveryPath: destination.path,
        namingTemplate: namingTemplate || null,
      })

      setDeliveryJobs((prev) => [...prev, job])

      // Auto-start the delivery
      await invoke('start_delivery', { jobId: job.id })
    } catch (err) {
      console.error('Failed to create delivery:', err)
    }
  }

  async function removeJob(jobId: string) {
    try {
      await invoke('remove_delivery_job', { jobId })
      setDeliveryJobs((prev) => prev.filter((j) => j.id !== jobId))
    } catch (err) {
      console.error('Failed to remove job:', err)
    }
  }

  function formatBytes(bytes: number): string {
    if (bytes === 0) return '0 B'
    const k = 1024
    const sizes = ['B', 'KB', 'MB', 'GB']
    const i = Math.floor(Math.log(bytes) / Math.log(k))
    return Math.round((bytes / Math.pow(k, i)) * 100) / 100 + ' ' + sizes[i]
  }

  function getStatusClass(status: string): string {
    switch (status) {
      case 'pending':
        return 'status-pending'
      case 'inprogress':
        return 'status-inprogress'
      case 'completed':
        return 'status-completed'
      case 'failed':
        return 'status-failed'
      default:
        return ''
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
          <select
            value={selectedProject?.id || ''}
            onChange={(e) => {
              const project = projects.find((p) => p.id === e.target.value)
              setSelectedProject(project || null)
            }}
            className="project-select"
          >
            <option value="">Choose a project...</option>
            {projects.map((project) => (
              <option key={project.id} value={project.id}>
                {project.name} - {project.clientName}
              </option>
            ))}
          </select>
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
                      onClick={() => createDelivery(dest)}
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
                      <button onClick={() => removeJob(job.id)} className="btn-remove">
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
