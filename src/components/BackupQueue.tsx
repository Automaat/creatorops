import { useEffect, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { formatBytes, formatSpeed, formatETA } from '../utils/formatting'
import type { BackupJob, BackupProgress } from '../types'

const QUEUE_REFRESH_INTERVAL = 30000

export function BackupQueue() {
  const [jobs, setJobs] = useState<BackupJob[]>([])
  const [progress, setProgress] = useState<Map<string, BackupProgress>>(new Map())

  useEffect(() => {
    loadQueue()

    const unlistenProgress = listen<BackupProgress>('backup-progress', (event) => {
      setProgress((prev) => new Map(prev).set(event.payload.jobId, event.payload))
    })

    const unlistenJobUpdate = listen<BackupJob>('backup-job-updated', (event) => {
      setJobs((prev) => prev.map((j) => (j.id === event.payload.id ? event.payload : j)))
    })

    // Occasional refresh to catch any missed events
    const interval = setInterval(loadQueue, QUEUE_REFRESH_INTERVAL)

    return () => {
      unlistenProgress.then((fn) => fn())
      unlistenJobUpdate.then((fn) => fn())
      clearInterval(interval)
    }
  }, [])

  async function loadQueue() {
    try {
      const data = await invoke<BackupJob[]>('get_backup_queue')
      setJobs(data)
    } catch (err) {
      console.error('Failed to load backup queue:', err)
    }
  }

  async function startBackup(jobId: string) {
    try {
      await invoke('start_backup', { jobId })
      await loadQueue()
    } catch (err) {
      console.error('Failed to start backup:', err)
    }
  }

  async function cancelBackup(jobId: string) {
    try {
      await invoke('cancel_backup', { jobId })
      await loadQueue()
    } catch (err) {
      console.error('Failed to cancel backup:', err)
    }
  }

  async function removeJob(jobId: string) {
    try {
      await invoke('remove_backup_job', { jobId })
      await loadQueue()
    } catch (err) {
      console.error('Failed to remove job:', err)
    }
  }

  const pendingJobs = jobs.filter((j) => j.status === 'pending')
  const activeJobs = jobs.filter((j) => j.status === 'inprogress')
  const completedJobs = jobs.filter((j) => ['completed', 'failed', 'cancelled'].includes(j.status))

  return (
    <div className="backup-queue">
      <div className="backup-queue-header">
        <h1>Backup Queue</h1>
      </div>

      {activeJobs.length > 0 && (
        <section className="backup-section">
          <h2>In Progress</h2>
          <div className="backup-list">
            {activeJobs.map((job) => {
              const jobProgress = progress.get(job.id)
              const percentage =
                job.totalBytes > 0 ? (job.bytesTransferred / job.totalBytes) * 100 : 0

              return (
                <div key={job.id} className="backup-job">
                  <div className="backup-job-header">
                    <div className="backup-job-info">
                      <h3>{job.projectName}</h3>
                      <p className="backup-destination">→ {job.destinationName}</p>
                    </div>
                    <div className="backup-job-status status-inprogress">In Progress</div>
                  </div>

                  {jobProgress && (
                    <div className="backup-progress">
                      <div className="progress-info">
                        <span className="progress-file">{jobProgress.fileName}</span>
                        <span className="progress-count">
                          {jobProgress.currentFile} / {jobProgress.totalFiles} files
                        </span>
                      </div>

                      <div className="progress-bar">
                        <div className="progress-fill" style={{ width: `${percentage}%` }} />
                      </div>

                      <div className="progress-stats">
                        <span>
                          {formatBytes(job.bytesTransferred)} / {formatBytes(job.totalBytes)}
                        </span>
                        <span>{formatSpeed(jobProgress.speed)}</span>
                        <span>ETA: {formatETA(jobProgress.eta)}</span>
                      </div>
                    </div>
                  )}
                </div>
              )
            })}
          </div>
        </section>
      )}

      {pendingJobs.length > 0 && (
        <section className="backup-section">
          <h2>Pending</h2>
          <div className="backup-list">
            {pendingJobs.map((job) => (
              <div key={job.id} className="backup-job">
                <div className="backup-job-header">
                  <div className="backup-job-info">
                    <h3>{job.projectName}</h3>
                    <p className="backup-destination">→ {job.destinationName}</p>
                  </div>
                  <div className="backup-job-status status-pending">Pending</div>
                </div>

                <div className="backup-job-meta">
                  <span>{job.totalFiles} files</span>
                  <span>{formatBytes(job.totalBytes)}</span>
                </div>

                <div className="backup-job-actions">
                  <button onClick={() => startBackup(job.id)} className="btn-primary">
                    Start Backup
                  </button>
                  <button onClick={() => cancelBackup(job.id)} className="btn-secondary">
                    Cancel
                  </button>
                </div>
              </div>
            ))}
          </div>
        </section>
      )}

      {completedJobs.length > 0 && (
        <section className="backup-section">
          <h2>Completed</h2>
          <div className="backup-list">
            {completedJobs.map((job) => (
              <div key={job.id} className="backup-job">
                <div className="backup-job-header">
                  <div className="backup-job-info">
                    <h3>{job.projectName}</h3>
                    <p className="backup-destination">→ {job.destinationName}</p>
                  </div>
                  <div className={`backup-job-status status-${job.status}`}>
                    {job.status === 'completed' && 'Completed'}
                    {job.status === 'failed' && 'Failed'}
                    {job.status === 'cancelled' && 'Cancelled'}
                  </div>
                </div>

                <div className="backup-job-meta">
                  <span>{job.filesCopied} files copied</span>
                  {job.filesSkipped > 0 && (
                    <span className="text-warning">{job.filesSkipped} files skipped</span>
                  )}
                  <span>{formatBytes(job.bytesTransferred)}</span>
                </div>

                {job.errorMessage && <div className="backup-error">{job.errorMessage}</div>}

                <div className="backup-job-actions">
                  <button onClick={() => removeJob(job.id)} className="btn-secondary">
                    Remove
                  </button>
                </div>
              </div>
            ))}
          </div>
        </section>
      )}

      {jobs.length === 0 && (
        <div className="empty-state">
          <p>No backup jobs in queue</p>
          <p className="empty-state-hint">Create a backup from the Projects view</p>
        </div>
      )}
    </div>
  )
}
