import { useCallback, useEffect, useState, useRef } from 'react'
import { invoke } from '@tauri-apps/api/core'
import type { Project } from '../types'

interface CommandPaletteProps {
  isOpen: boolean
  onClose: () => void
  onSelectProject: (projectId: string) => void
}

export function CommandPalette({ isOpen, onClose, onSelectProject }: CommandPaletteProps) {
  const [projects, setProjects] = useState<Project[]>([])
  const [searchQuery, setSearchQuery] = useState('')
  const [selectedIndex, setSelectedIndex] = useState(0)
  const inputRef = useRef<HTMLInputElement>(null)
  const selectedItemRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    if (isOpen) {
      loadProjects()
      setSearchQuery('')
      setSelectedIndex(0)
      // Focus input after render
      setTimeout(() => inputRef.current?.focus(), 0)
    }
  }, [isOpen])

  const loadProjects = async () => {
    try {
      const projectList = await invoke<Project[]>('list_projects')
      setProjects(projectList)
    } catch (error) {
      console.error('Failed to load projects:', error)
    }
  }

  const filteredProjects = projects.filter((project) => {
    const query = searchQuery.toLowerCase()
    return (
      project.name.toLowerCase().includes(query) ||
      project.clientName.toLowerCase().includes(query) ||
      project.shootType.toLowerCase().includes(query) ||
      project.date.toLowerCase().includes(query)
    )
  })

  useEffect(() => {
    setSelectedIndex(0)
  }, [searchQuery])

  useEffect(() => {
    selectedItemRef.current?.scrollIntoView({ block: 'nearest', behavior: 'smooth' })
  }, [selectedIndex])

  const handleSelectProject = useCallback(
    (projectId: string) => {
      onSelectProject(projectId)
      onClose()
    },
    [onSelectProject, onClose]
  )

  useEffect(() => {
    if (!isOpen) return

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        onClose()
      } else if (e.key === 'ArrowDown') {
        e.preventDefault()
        setSelectedIndex((prev) => Math.min(prev + 1, filteredProjects.length - 1))
      } else if (e.key === 'ArrowUp') {
        e.preventDefault()
        setSelectedIndex((prev) => Math.max(prev - 1, 0))
      } else if (e.key === 'Enter' && filteredProjects[selectedIndex]) {
        e.preventDefault()
        handleSelectProject(filteredProjects[selectedIndex].id)
      }
    }

    document.addEventListener('keydown', handleKeyDown)
    return () => document.removeEventListener('keydown', handleKeyDown)
  }, [isOpen, selectedIndex, filteredProjects, onClose, handleSelectProject])

  function getStatusColor(status: string): string {
    switch (status) {
      case 'New':
        return 'status-new'
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

  if (!isOpen) return null

  return (
    <div className="command-palette-overlay" onClick={onClose}>
      <div className="command-palette" onClick={(e) => e.stopPropagation()}>
        <div className="command-palette-header">
          <input
            ref={inputRef}
            type="text"
            className="command-palette-input"
            placeholder="Search projects..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
          />
        </div>

        <div className="command-palette-results">
          {filteredProjects.length > 0 ? (
            filteredProjects.map((project, index) => (
              <div
                key={project.id}
                ref={index === selectedIndex ? selectedItemRef : null}
                className={`command-palette-item ${index === selectedIndex ? 'selected' : ''}`}
                onClick={() => handleSelectProject(project.id)}
                onMouseEnter={() => setSelectedIndex(index)}
              >
                <div className="command-palette-item-content">
                  <div className="command-palette-item-header">
                    <span className="command-palette-item-title">{project.name}</span>
                    <span className={`project-status ${getStatusColor(project.status)}`}>
                      {project.status}
                    </span>
                  </div>
                  <div className="command-palette-item-meta">
                    {project.clientName} · {project.date} · {project.shootType}
                  </div>
                </div>
              </div>
            ))
          ) : (
            <div className="command-palette-empty">
              <p className="text-secondary">
                {searchQuery ? 'No projects found' : 'No projects available'}
              </p>
            </div>
          )}
        </div>

        <div className="command-palette-footer">
          <span className="command-palette-hint">
            <kbd>↑</kbd>
            <kbd>↓</kbd> Navigate · <kbd>↵</kbd> Select · <kbd>Esc</kbd> Close
          </span>
        </div>
      </div>
    </div>
  )
}
