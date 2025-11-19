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
