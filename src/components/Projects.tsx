import { useEffect, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import type { Project, BackupDestination } from '../types'

export function Projects() {
  const [projects, setProjects] = useState<Project[]>([])
  const [destinations, setDestinations] = useState<BackupDestination[]>([])
  const [selectedProject, setSelectedProject] = useState<Project | null>(null)
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    loadProjects()
    loadDestinations()
  }, [])

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

  async function loadDestinations() {
    try {
      // Load from localStorage for now (will add Rust backend later)
      const stored = localStorage.getItem('backup_destinations')
      if (stored) {
        setDestinations(JSON.parse(stored))
      }
    } catch (err) {
      console.error('Failed to load destinations:', err)
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
      console.log(`Backup queued for ${project.name} to ${destination.name}`)
    } catch (err) {
      console.error('Failed to queue backup:', err)
    }
  }

  function getStatusColor(status: string): string {
    switch (status) {
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

  if (loading) {
    return <div className="loading">Loading projects...</div>
  }

  if (selectedProject) {
    return (
      <div className="project-detail">
        <div className="project-detail-header">
          <button onClick={() => setSelectedProject(null)} className="btn-back">
            ← Back to Projects
          </button>
          <h1>{selectedProject.name}</h1>
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
            <span className="info-label">Location:</span>
            <span className="folder-path">{selectedProject.folderPath}</span>
          </div>
        </div>

        <section className="project-actions">
          <h2>Actions</h2>

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
        </section>
      </div>
    )
  }

  return (
    <div className="projects">
      <div className="projects-header">
        <h1>Projects</h1>
      </div>

      {projects.length === 0 ? (
        <div className="empty-state">
          <p>No projects yet</p>
          <p className="empty-state-hint">Import files from an SD card to create a project</p>
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
  )
}
