import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { ProjectStatus } from '../types'
import type { Project } from '../types'
import { isOverdue, sortProjects } from '../utils/project'
import { formatDisplayDate } from '../utils/formatting'
import { useNotification } from '../hooks/useNotification'
import { CreateProject } from './CreateProject'

interface DashboardProps {
  isActive?: boolean
  onProjectClick?: (projectId: string) => void
}

export function Dashboard({ isActive, onProjectClick }: DashboardProps) {
  const { error: showError } = useNotification()
  const isActiveRef = useRef(isActive ?? false)
  useEffect(() => {
    isActiveRef.current = isActive ?? false
  }, [isActive])
  const [projects, setProjects] = useState<Project[]>([])
  const [loading, setLoading] = useState(true)
  const [showCreateProject, setShowCreateProject] = useState(false)

  const loadData = useCallback(async () => {
    try {
      setLoading(true)
      const projectList = await invoke<Project[]>('list_projects')
      setProjects(projectList)
    } catch (error) {
      console.error('Failed to load dashboard data:', error)
      if (isActiveRef.current) showError('Failed to load dashboard data')
    } finally {
      setLoading(false)
    }
  }, [showError])

  useEffect(() => {
    void loadData()
  }, [loadData])

  // One-time migration: extract clients from existing projects
  useEffect(() => {
    const MIGRATION_KEY = 'clients_migrated_v1'
    if (localStorage.getItem(MIGRATION_KEY)) return

    invoke('migrate_clients_from_projects')
      .then(() => {
        localStorage.setItem(MIGRATION_KEY, '1')
      })
      .catch((err: unknown) => {
        console.error('Client migration failed:', err)
      })
  }, [])

  function getStatusColor(status: string): string {
    switch (status) {
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

  const activeProjects = useMemo(
    () => sortProjects(projects.filter((p) => p.status !== ProjectStatus.Archived)),
    [projects]
  )

  const handleProjectCreated = (project: Project) => {
    setShowCreateProject(false)
    void loadData() // Refresh the dashboard
    onProjectClick?.(project.id) // Navigate to the new project
  }

  if (loading) {
    return <div className="loading">Loading...</div>
  }

  return (
    <>
      <div className="content-header">
        <h1>Dashboard</h1>
        <p className="text-secondary">Overview of your photography workflow</p>
        <button className="btn-primary" onClick={() => setShowCreateProject(true)}>
          New Project
        </button>
      </div>
      <div className="content-body">
        <div className="flex flex-col gap-xxl">
          <section>
            <div className="section-label">MY PROJECTS</div>
            <h2>Active projects</h2>
            <div className="project-list">
              {activeProjects.length === 0 ? (
                <div className="empty-state">
                  <p className="text-secondary">No active projects</p>
                </div>
              ) : (
                activeProjects.map((project) => (
                  <div
                    key={project.id}
                    className={`project-list-item ${onProjectClick ? 'clickable' : ''}`}
                    onClick={() => onProjectClick?.(project.id)}
                    role="button"
                    tabIndex={0}
                  >
                    <div className="project-list-content">
                      <div>
                        <h3>{project.name}</h3>
                        <p className="text-secondary text-sm">
                          {project.date}
                          {project.deadline && (
                            <>
                              {' · '}
                              <span className={isOverdue(project.deadline) ? 'text-overdue' : ''}>
                                Due {formatDisplayDate(project.deadline)}
                              </span>
                            </>
                          )}
                        </p>
                      </div>
                      <span className={`project-status ${getStatusColor(project.status)}`}>
                        {project.status}
                      </span>
                    </div>
                  </div>
                ))
              )}
            </div>
          </section>

          <section>
            <h2>Quick Stats</h2>
            <div className="stats-grid">
              <div className="stat-card">
                <div className="stat-value">{projects.length}</div>
                <div className="stat-label">Total Projects</div>
              </div>
              <div className="stat-card">
                <div className="stat-value">{activeProjects.length}</div>
                <div className="stat-label">Active Projects</div>
              </div>
            </div>
          </section>
        </div>
      </div>

      {showCreateProject && (
        <div
          className="dialog-overlay"
          onClick={() => setShowCreateProject(false)}
          role="presentation"
        >
          <div
            className="dialog"
            onClick={(e) => e.stopPropagation()}
            onKeyDown={(e) => e.key === 'Escape' && setShowCreateProject(false)}
            role="dialog"
          >
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
