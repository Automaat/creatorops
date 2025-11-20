import { useEffect, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import type { Project, BackupDestination, ArchiveJob } from '../types'
import { CreateProject } from './CreateProject'

interface ProjectsProps {
  initialSelectedProjectId?: string | null
}

export function Projects({ initialSelectedProjectId }: ProjectsProps) {
  const [projects, setProjects] = useState<Project[]>([])
  const [destinations, setDestinations] = useState<BackupDestination[]>([])
  const [selectedProject, setSelectedProject] = useState<Project | null>(null)
  const [loading, setLoading] = useState(true)
  const [archiveLocation, setArchiveLocation] = useState('')
  const [showArchiveDialog, setShowArchiveDialog] = useState(false)
  const [showCreateProject, setShowCreateProject] = useState(false)
  const [showDeleteDialog, setShowDeleteDialog] = useState(false)

  useEffect(() => {
    loadProjects()
    loadDestinations()
    loadArchiveLocation()
  }, [])

  // Handle initial project selection from navigation
  useEffect(() => {
    if (initialSelectedProjectId && projects.length > 0) {
      const project = projects.find((p) => p.id === initialSelectedProjectId)
      if (project) {
        setSelectedProject(project)
      }
    }
  }, [initialSelectedProjectId, projects])

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

      // Refresh projects list after a delay
      setTimeout(() => {
        loadProjects()
      }, 1000)
    } catch (err) {
      console.error('Failed to archive project:', err)
      alert(`Failed to archive project: ${err}`)
    }
  }

  function handleProjectCreated(project: Project) {
    setShowCreateProject(false)
    setProjects([project, ...projects])
  }

  async function deleteProject() {
    if (!selectedProject) return

    try {
      await invoke('delete_project', { projectId: selectedProject.id })
      setShowDeleteDialog(false)
      setSelectedProject(null)
      loadProjects()
    } catch (err) {
      console.error('Failed to delete project:', err)
      alert(`Failed to delete project: ${err}`)
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
          <div className="dialog-overlay" onClick={() => setShowDeleteDialog(false)}>
            <div className="dialog" onClick={(e) => e.stopPropagation()}>
              <h2>Delete Project</h2>
              <p>
                Are you sure you want to delete <strong>{selectedProject.name}</strong>? This will
                permanently delete the project folder and all its contents.
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
            </div>
          </div>
        )}
      </div>
    )
  }

  return (
    <>
      <div className="projects">
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
