// Type definitions

interface SDCard {
  name: string
  path: string
  size: number
  freeSpace: number
  fileCount: number
  deviceType: string
  isRemovable: boolean
}

interface Project {
  id: string
  name: string
  clientName: string
  date: string
  shootType: string
  status: ProjectStatus
  folderPath: string
  createdAt: string
  updatedAt: string
  deadline?: string
}

enum ProjectStatus {
  New = 'New',
  Importing = 'Importing',
  Editing = 'Editing',
  Delivered = 'Delivered',
  Archived = 'Archived',
}

interface ImportProgress {
  filesCopied: number
  totalFiles: number
  currentFile: string
}

interface FileInfo {
  name: string
  path: string
  size: number
  modified: string
  type: string
}

interface ImportHistory {
  id: string
  projectId: string
  projectName: string
  sourcePath: string
  destinationPath: string
  filesCopied: number
  filesSkipped: number
  totalBytes: number
  photosCopied: number
  videosCopied: number
  startedAt: string
  completedAt: string
  status: 'success' | 'partial' | 'failed'
  errorMessage?: string
}

interface CopyResult {
  success: boolean
  error?: string
  filesCopied: number
  filesSkipped: number
  skippedFiles: string[]
  totalBytes: number
  photosCopied: number
  videosCopied: number
}

interface BackupDestination {
  id: string
  name: string
  path: string
  enabled: boolean
  createdAt: string
}

// Note: Rust enum uses PascalCase (InProgress) but serde renames to lowercase for JSON
type BackupStatus = 'pending' | 'inprogress' | 'completed' | 'failed' | 'cancelled'

interface BackupJob {
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

interface BackupProgress {
  jobId: string
  fileName: string
  currentFile: number
  totalFiles: number
  bytesTransferred: number
  totalBytes: number
  speed: number
  eta: number
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
  status: BackupStatus
  errorMessage?: string
}

type JobStatus = 'pending' | 'inprogress' | 'completed' | 'failed'

interface DeliveryJob {
  id: string
  projectId: string
  projectName: string
  selectedFiles: string[]
  deliveryPath: string
  namingTemplate?: string
  status: JobStatus
  totalFiles: number
  filesCopied: number
  totalBytes: number
  bytesTransferred: number
  createdAt: string
  startedAt?: string
  completedAt?: string
  errorMessage?: string
  manifestPath?: string
}

interface DeliveryDestination {
  id: string
  name: string
  path: string
  enabled: boolean
  createdAt: string
}

interface DeliveryProgress {
  jobId: string
  fileName: string
  currentFile: number
  totalFiles: number
  bytesTransferred: number
  totalBytes: number
  speed: number
  eta: number
}

interface ArchiveJob {
  id: string
  projectId: string
  projectName: string
  sourcePath: string
  archivePath: string
  compress: boolean
  compressionFormat?: 'zip' | 'tar'
  status: JobStatus
  totalFiles: number
  filesArchived: number
  totalBytes: number
  bytesTransferred: number
  createdAt: string
  startedAt?: string
  completedAt?: string
  errorMessage?: string
}

interface ProjectFile {
  name: string
  path: string
  size: number
  modified: string
  type: string
  relativePath: string
}

export type {
  SDCard,
  Project,
  ImportProgress,
  FileInfo,
  ImportHistory,
  CopyResult,
  BackupDestination,
  BackupStatus,
  BackupJob,
  BackupProgress,
  BackupHistory,
  JobStatus,
  DeliveryJob,
  DeliveryDestination,
  DeliveryProgress,
  ArchiveJob,
  ProjectFile,
}

export { ProjectStatus }
