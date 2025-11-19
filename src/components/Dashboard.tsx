import { useEffect, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import type { Project, ImportHistory } from '../types'

interface DashboardProps {
  onProjectClick?: (projectId: string) => void
}

export function Dashboard({ onProjectClick }: DashboardProps) {
  const [projects, setProjects] = useState<Project[]>([])
  const [recentImports, setRecentImports] = useState<ImportHistory[]>([])
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    loadData()
  }, [])

  async function loadData() {
    try {
      setLoading(true)
      const [projectList, importList] = await Promise.all([
        invoke<Project[]>('list_projects'),
        invoke<ImportHistory[]>('get_import_history'),
      ])
      setProjects(projectList)
      setRecentImports(importList.slice(0, 5)) // Show last 5
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

  function formatBytes(bytes: number): string {
    if (!Number.isFinite(bytes) || bytes <= 0) return '0 B'
    const k = 1024
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB']
    const i = Math.floor(Math.log(bytes) / Math.log(k))
    return `${(bytes / Math.pow(k, i)).toFixed(2)} ${sizes[i]}`
  }

  function formatDate(dateStr: string): string {
    try {
      const date = new Date(dateStr)
      return date.toLocaleDateString()
    } catch {
      return dateStr
    }
  }

  const activeProjects = projects.filter((p) => p.status !== 'Archived')

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
                        <p className="text-secondary text-sm">
                          {project.clientName} · {project.date} · {project.shootType}
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
            <h2>Recent Imports</h2>
            <div className="flex flex-col gap-md">
              {recentImports.length === 0 ? (
                <div className="card">
                  <p className="text-secondary">No recent imports</p>
                </div>
              ) : (
                recentImports.map((imp, idx) => (
                  <div key={idx} className="card">
                    <div className="flex flex-col gap-xs">
                      <div className="flex flex-between">
                        <h4>{imp.projectName}</h4>
                        <span className="text-sm text-secondary">{formatDate(imp.startedAt)}</span>
                      </div>
                      <p className="text-sm text-secondary">
                        {imp.filesCopied} files · {formatBytes(imp.totalBytes)}
                        {imp.errorMessage && (
                          <span className="text-error"> · Error: {imp.errorMessage}</span>
                        )}
                      </p>
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
                <div className="flex flex-between">
                  <span className="text-secondary">Recent Imports:</span>
                  <span className="font-medium">{recentImports.length}</span>
                </div>
              </div>
            </div>
          </section>
        </div>
      </div>
    </>
  )
}
