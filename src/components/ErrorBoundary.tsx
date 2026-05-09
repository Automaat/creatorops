import { Component, useCallback, useEffect, useRef, useState } from 'react'
import type { ErrorInfo, ReactNode } from 'react'

interface ErrorBoundaryProps {
  children: ReactNode
  name?: string
  isActive?: boolean
}

interface BoundaryState {
  hasError: boolean
  error: Error | undefined
}

interface BoundaryInnerProps {
  children: ReactNode
  name?: string
  onReset: () => void
  onError: () => void
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
    this.props.onError()
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

export function ErrorBoundary({ children, name, isActive }: ErrorBoundaryProps) {
  const [resetKey, setResetKey] = useState(0)
  const hasErrorRef = useRef(false)
  const prevIsActiveRef = useRef(isActive)

  const handleReset = useCallback(() => {
    hasErrorRef.current = false
    setResetKey((k) => k + 1)
  }, [])

  const handleError = useCallback(() => {
    hasErrorRef.current = true
  }, [])

  useEffect(() => {
    if (isActive && !prevIsActiveRef.current && hasErrorRef.current) {
      hasErrorRef.current = false
      setResetKey((k) => k + 1)
    }
    prevIsActiveRef.current = isActive
  }, [isActive])

  return (
    <ErrorBoundaryInner key={resetKey} name={name} onReset={handleReset} onError={handleError}>
      {children}
    </ErrorBoundaryInner>
  )
}
