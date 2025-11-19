// Type definitions

export interface SDCard {
  name: string
  path: string
  size: number
  freeSpace: number
  fileCount: number
  deviceType: string
  isRemovable: boolean
}

export interface Project {
  id: string
  name: string
  clientName: string
  date: string
  shootType: string
  status: ProjectStatus
  folderPath: string
  createdAt: string
  updatedAt: string
}

export enum ProjectStatus {
  Importing = 'Importing',
  Editing = 'Editing',
  Delivered = 'Delivered',
  Archived = 'Archived',
}

export interface ImportProgress {
  fileName: string
  currentFile: number
  totalFiles: number
  bytesTransferred: number
  totalBytes: number
  speed: number
  eta: number
}

export interface FileInfo {
  name: string
  path: string
  size: number
  modified: string
  type: string
}

export interface ImportHistory {
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

export interface CopyResult {
  success: boolean
  error?: string
  filesCopied: number
  filesSkipped: number
  skippedFiles: string[]
}

export interface BackupDestination {
  id: string
  name: string
  path: string
  enabled: boolean
  createdAt: string
}

// Note: Rust enum uses PascalCase (InProgress) but serde renames to lowercase for JSON
export type BackupStatus = 'pending' | 'inprogress' | 'completed' | 'failed' | 'cancelled'

export interface BackupJob {
  id: string
  projectId: string
  projectName: string
  sourcePath: string
  destinationId: string
  destinationName: string
  destinationPath: string
  status: BackupStatus
  totalFiles: number
  filesCopied: number
  filesSkipped: number
  totalBytes: number
  bytesTransferred: number
  createdAt: string
  startedAt?: string
  completedAt?: string
  errorMessage?: string
}

export interface BackupProgress {
  jobId: string
  fileName: string
  currentFile: number
  totalFiles: number
  bytesTransferred: number
  totalBytes: number
  speed: number
  eta: number
}

export interface BackupHistory {
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
  status: BackupStatus
  errorMessage?: string
}
