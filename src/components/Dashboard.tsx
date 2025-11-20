import { useEffect, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import type { Project } from '../types'
import { formatProjectInfo } from '../utils/project'

interface DashboardProps {
  onProjectClick?: (projectId: string) => void
}

export function Dashboard({ onProjectClick }: DashboardProps) {
  const [projects, setProjects] = useState<Project[]>([])
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    loadData()
  }, [])

  async function loadData() {
    try {
      setLoading(true)
      const projectList = await invoke<Project[]>('list_projects')
      setProjects(projectList)
    } catch (err) {
      console.error('Failed to load dashboard data:', err)
    } finally {
      setLoading(false)
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

  const activeProjects = projects
    .filter((p) => p.status !== 'Archived')
    .sort((a, b) => {
      // Projects with deadlines come first, sorted by deadline (earliest first)
      if (a.deadline && b.deadline) {
        return new Date(a.deadline).getTime() - new Date(b.deadline).getTime()
      }
      if (a.deadline) return -1
      if (b.deadline) return 1
      // No deadline: sort by updated date descending
      return b.updatedAt.localeCompare(a.updatedAt)
    })

  if (loading) {
    return <div className="loading">Loading...</div>
  }

  return (
    <>
      <div className="content-header">
        <h1>Dashboard</h1>
        <p className="text-secondary">Overview of your photography workflow</p>
      </div>
      <div className="content-body">
        <div className="flex flex-col gap-xl">
          <section>
            <h2>Active Projects</h2>
            <div className="flex flex-col gap-md">
              {activeProjects.length === 0 ? (
                <div className="card">
                  <p className="text-secondary">No active projects</p>
                </div>
              ) : (
                activeProjects.map((project) => (
                  <div
                    key={project.id}
                    className={`card ${onProjectClick ? 'card-clickable' : ''}`}
                    onClick={() => onProjectClick?.(project.id)}
                  >
                    <div className="flex flex-between">
                      <div>
                        <h3>{project.name}</h3>
                        <p className="text-secondary text-sm">{formatProjectInfo(project)}</p>
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
            <div className="card">
              <div className="flex flex-col gap-md">
                <div className="flex flex-between">
                  <span className="text-secondary">Total Projects:</span>
                  <span className="font-medium">{projects.length}</span>
                </div>
                <div className="flex flex-between">
                  <span className="text-secondary">Active Projects:</span>
                  <span className="font-medium">{activeProjects.length}</span>
                </div>
              </div>
            </div>
          </section>
        </div>
      </div>
    </>
  )
}
