import { Component, useCallback, useState } from 'react'
import type { ErrorInfo, ReactNode } from 'react'

interface ErrorBoundaryProps {
  children: ReactNode
  name?: string
}

interface BoundaryState {
  hasError: boolean
  error: Error | undefined
}

interface BoundaryInnerProps {
  children: ReactNode
  name?: string
  onReset: () => void
}

class ErrorBoundaryInner extends Component<BoundaryInnerProps, BoundaryState> {
  constructor(props: BoundaryInnerProps) {
    super(props)
    this.state = { error: undefined, hasError: false }
  }

  static getDerivedStateFromError(error: Error): BoundaryState {
    return { error, hasError: true }
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    const label = this.props.name ? ` (${this.props.name})` : ''
    console.error(`[ErrorBoundary${label}]`, error, info.componentStack)
  }

  render() {
    const { error, hasError } = this.state
    return hasError && error ? (
      <div className="error-boundary-fallback">
        <h2 className="error-boundary-title">Something went wrong</h2>
        {this.props.name && <p className="error-boundary-section">in {this.props.name}</p>}
        <p className="error-boundary-message">{error.message}</p>
        <button type="button" className="error-boundary-reset" onClick={this.props.onReset}>
          Try again
        </button>
      </div>
    ) : (
      this.props.children
    )
  }
}

export function ErrorBoundary({ children, name }: ErrorBoundaryProps) {
  const [resetKey, setResetKey] = useState(0)
  const handleReset = useCallback(() => setResetKey((k) => k + 1), [])
  return (
    <ErrorBoundaryInner key={resetKey} name={name} onReset={handleReset}>
      {children}
    </ErrorBoundaryInner>
  )
}
