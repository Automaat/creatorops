import { useEffect } from 'react'

export interface KeyboardShortcut {
  key: string
  metaKey?: boolean
  ctrlKey?: boolean
  shiftKey?: boolean
  altKey?: boolean
  description: string
  action: () => void
}

export function useKeyboardShortcuts(shortcuts: KeyboardShortcut[], enabled = true) {
  useEffect(() => {
    if (!enabled) return

    const handleKeyDown = (e: KeyboardEvent) => {
      for (const shortcut of shortcuts) {
        const metaMatch = shortcut.metaKey === undefined || shortcut.metaKey === e.metaKey
        const ctrlMatch = shortcut.ctrlKey === undefined || shortcut.ctrlKey === e.ctrlKey
        const shiftMatch = shortcut.shiftKey === undefined || shortcut.shiftKey === e.shiftKey
        const altMatch = shortcut.altKey === undefined || shortcut.altKey === e.altKey
        const keyMatch = shortcut.key.toLowerCase() === e.key.toLowerCase()

        if (metaMatch && ctrlMatch && shiftMatch && altMatch && keyMatch) {
          e.preventDefault()
          shortcut.action()
          break
        }
      }
    }

    window.addEventListener('keydown', handleKeyDown)
    return () => window.removeEventListener('keydown', handleKeyDown)
  }, [shortcuts, enabled])
}

export const GLOBAL_SHORTCUTS: Omit<KeyboardShortcut, 'action'>[] = [
  { key: ',', metaKey: true, description: 'Open Settings' },
  { key: '/', metaKey: true, description: 'Show Keyboard Shortcuts' },
  { key: '1', metaKey: true, description: 'Go to Dashboard' },
  { key: '2', metaKey: true, description: 'Go to Import' },
  { key: '3', metaKey: true, description: 'Go to Projects' },
  { key: '4', metaKey: true, description: 'Go to Backup Queue' },
  { key: '5', metaKey: true, description: 'Go to Delivery' },
  { key: '6', metaKey: true, description: 'Go to History' },
  { key: 'r', metaKey: true, description: 'Refresh SD Cards' },
]
