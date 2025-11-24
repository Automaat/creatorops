import '@testing-library/jest-dom'

// Mock Tauri internals for tests
Object.defineProperty(window, '__TAURI_INTERNALS__', {
  value: {
    transformCallback: <T>(callback: T): T => {
      return callback
    },
  },
  writable: true,
})
