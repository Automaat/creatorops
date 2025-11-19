// Type definitions

export interface SDCard {
  name: string
  path: string
  size: number
  freeSpace: number
  fileCount: number
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
