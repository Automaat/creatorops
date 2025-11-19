import { useEffect, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { formatBytes, formatDate } from '../utils/formatting'
import { useNotification } from '../hooks/useNotification'
import '../styles/history.css'

interface ImportHistory {
  id: string
  projectId: string
  projectName: string
  sourcePath: string
  destinationPath: string
  filesCopied: number
  filesSkipped: number
  totalBytes: number
  startedAt: string
  completedAt: string
  status: 'success' | 'partial' | 'failed'
  errorMessage?: string
}

interface BackupHistory {
  id: string
  projectId: string
  projectName: string
  destinationName: string
  destinationPath: string
  filesCopied: number
  filesSkipped: number
  totalBytes: number
  startedAt: string
  completedAt: string
  status: 'completed' | 'failed' | 'cancelled'
  errorMessage?: string
}

type HistoryType = 'import' | 'backup'

export function History() {
  const [historyType, setHistoryType] = useState<HistoryType>('import')
  const [importHistory, setImportHistory] = useState<ImportHistory[]>([])
  const [backupHistory, setBackupHistory] = useState<BackupHistory[]>([])
  const [isLoading, setIsLoading] = useState(false)
  const { error } = useNotification()

  useEffect(() => {
    async function loadHistory() {
      setIsLoading(true)
      try {
        if (historyType === 'import') {
          const history = await invoke<ImportHistory[]>('get_import_history', { limit: 100 })
          setImportHistory(history)
        } else {
          const history = await invoke<BackupHistory[]>('get_backup_history')
          setBackupHistory(history)
        }
      } catch (err) {
        console.error('Failed to load history:', err)
        error('Failed to load history')
      } finally {
        setIsLoading(false)
      }
    }
    loadHistory()
  }, [historyType, error])

  function getStatusClass(status: string): string {
    switch (status) {
      case 'success':
      case 'completed':
        return 'status-success'
      case 'partial':
        return 'status-warning'
      case 'failed':
      case 'cancelled':
        return 'status-error'
      default:
        return ''
    }
  }

  return (
    <>
      <div className="content-header">
        <h1>History</h1>
        <p className="text-secondary">View import and backup history</p>
      </div>
      <div className="content-body">
        <div className="history-tabs">
          <button
            className={`tab ${historyType === 'import' ? 'tab-active' : ''}`}
            onClick={() => setHistoryType('import')}
          >
            Imports ({importHistory.length})
          </button>
          <button
            className={`tab ${historyType === 'backup' ? 'tab-active' : ''}`}
            onClick={() => setHistoryType('backup')}
          >
            Backups ({backupHistory.length})
          </button>
        </div>

        {isLoading ? (
          <div className="empty-state">
            <p>Loading history...</p>
          </div>
        ) : historyType === 'import' ? (
          importHistory.length === 0 ? (
            <div className="empty-state">
              <p className="text-secondary">No import history yet</p>
            </div>
          ) : (
            <div className="history-list">
              {importHistory.map((item) => (
                <div key={item.id} className="history-item card">
                  <div className="history-header">
                    <div>
                      <h3 className="history-title">{item.projectName}</h3>
                      <p className="text-secondary text-sm">{formatDate(item.completedAt)}</p>
                    </div>
                    <span className={`status-badge ${getStatusClass(item.status)}`}>
                      {item.status}
                    </span>
                  </div>
                  <div className="history-details">
                    <div className="history-detail-item">
                      <span className="text-secondary">Source:</span>
                      <span>{item.sourcePath}</span>
                    </div>
                    <div className="history-detail-item">
                      <span className="text-secondary">Destination:</span>
                      <span>{item.destinationPath}</span>
                    </div>
                    <div className="history-stats">
                      <div className="stat">
                        <span className="stat-label">Files Copied</span>
                        <span className="stat-value">{item.filesCopied}</span>
                      </div>
                      {item.filesSkipped > 0 && (
                        <div className="stat">
                          <span className="stat-label">Files Skipped</span>
                          <span className="stat-value text-warning">{item.filesSkipped}</span>
                        </div>
                      )}
                      <div className="stat">
                        <span className="stat-label">Total Size</span>
                        <span className="stat-value">{formatBytes(item.totalBytes)}</span>
                      </div>
                    </div>
                    {item.errorMessage && (
                      <div className="error-message">
                        <span className="text-error">{item.errorMessage}</span>
                      </div>
                    )}
                  </div>
                </div>
              ))}
            </div>
          )
        ) : backupHistory.length === 0 ? (
          <div className="empty-state">
            <p className="text-secondary">No backup history yet</p>
          </div>
        ) : (
          <div className="history-list">
            {backupHistory.map((item) => (
              <div key={item.id} className="history-item card">
                <div className="history-header">
                  <div>
                    <h3 className="history-title">{item.projectName}</h3>
                    <p className="text-secondary text-sm">{formatDate(item.completedAt)}</p>
                  </div>
                  <span className={`status-badge ${getStatusClass(item.status)}`}>
                    {item.status}
                  </span>
                </div>
                <div className="history-details">
                  <div className="history-detail-item">
                    <span className="text-secondary">Destination:</span>
                    <span>
                      {item.destinationName} ({item.destinationPath})
                    </span>
                  </div>
                  <div className="history-stats">
                    <div className="stat">
                      <span className="stat-label">Files Copied</span>
                      <span className="stat-value">{item.filesCopied}</span>
                    </div>
                    {item.filesSkipped > 0 && (
                      <div className="stat">
                        <span className="stat-label">Files Skipped</span>
                        <span className="stat-value text-warning">{item.filesSkipped}</span>
                      </div>
                    )}
                    <div className="stat">
                      <span className="stat-label">Total Size</span>
                      <span className="stat-value">{formatBytes(item.totalBytes)}</span>
                    </div>
                  </div>
                  {item.errorMessage && (
                    <div className="error-message">
                      <span className="text-error">{item.errorMessage}</span>
                    </div>
                  )}
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </>
  )
}
