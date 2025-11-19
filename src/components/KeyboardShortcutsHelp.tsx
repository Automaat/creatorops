import { GLOBAL_SHORTCUTS } from '../hooks/useKeyboardShortcuts'
import '../styles/keyboard-shortcuts.css'

interface KeyboardShortcutsHelpProps {
  isOpen: boolean
  onClose: () => void
}

export function KeyboardShortcutsHelp({ isOpen, onClose }: KeyboardShortcutsHelpProps) {
  if (!isOpen) return null

  return (
    <div className="shortcuts-overlay" onClick={onClose}>
      <div className="shortcuts-modal" onClick={(e) => e.stopPropagation()}>
        <div className="shortcuts-header">
          <h2>Keyboard Shortcuts</h2>
          <button className="btn-icon" onClick={onClose}>
            ×
          </button>
        </div>
        <div className="shortcuts-content">
          <div className="shortcuts-section">
            <h3>Navigation</h3>
            <div className="shortcuts-list">
              {GLOBAL_SHORTCUTS.map((shortcut, index) => (
                <div key={index} className="shortcut-item">
                  <span className="shortcut-description">{shortcut.description}</span>
                  <div className="shortcut-keys">
                    {shortcut.metaKey && <kbd>⌘</kbd>}
                    {shortcut.ctrlKey && <kbd>Ctrl</kbd>}
                    {shortcut.shiftKey && <kbd>⇧</kbd>}
                    {shortcut.altKey && <kbd>⌥</kbd>}
                    <kbd>{shortcut.key.toUpperCase()}</kbd>
                  </div>
                </div>
              ))}
            </div>
          </div>
          <div className="shortcuts-section">
            <h3>General</h3>
            <div className="shortcuts-list">
              <div className="shortcut-item">
                <span className="shortcut-description">Close dialogs</span>
                <div className="shortcut-keys">
                  <kbd>Esc</kbd>
                </div>
              </div>
              <div className="shortcut-item">
                <span className="shortcut-description">Submit forms</span>
                <div className="shortcut-keys">
                  <kbd>Enter</kbd>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}
