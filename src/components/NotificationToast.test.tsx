import { describe, expect, it } from 'vitest'
import { render, screen } from '@testing-library/react'
import { userEvent } from '@testing-library/user-event'
import { NotificationToast } from './NotificationToast'
import { NotificationContext, NotificationProvider } from '../contexts/NotificationContext'
import { useContext } from 'react'

function TestWrapper({ children }: { children: React.ReactNode }) {
  return (
    <NotificationProvider>
      {children}
      <NotificationToast />
    </NotificationProvider>
  )
}

function NotificationTrigger() {
  const { success, error, warning, info } = useContext(NotificationContext)!

  return (
    <div>
      <button onClick={() => success('Success message')}>Trigger Success</button>
      <button onClick={() => error('Error message')}>Trigger Error</button>
      <button onClick={() => warning('Warning message')}>Trigger Warning</button>
      <button onClick={() => info('Info message')}>Trigger Info</button>
    </div>
  )
}

describe('notificationToast', () => {
  it('renders nothing when no notifications', () => {
    const { container } = render(
      <NotificationProvider>
        <NotificationToast />
      </NotificationProvider>
    )

    const notificationContainer = container.querySelector('.notification-container')
    expect(notificationContainer?.children).toHaveLength(0)
  })

  it('displays success notification', async () => {
    const user = userEvent.setup()

    render(
      <TestWrapper>
        <NotificationTrigger />
      </TestWrapper>
    )

    await user.click(screen.getByText('Trigger Success'))

    expect(screen.getByText('Success message')).toBeTruthy()
    expect(screen.getByText('✓')).toBeTruthy()
  })

  it('displays error notification', async () => {
    const user = userEvent.setup()

    render(
      <TestWrapper>
        <NotificationTrigger />
      </TestWrapper>
    )

    await user.click(screen.getByText('Trigger Error'))

    expect(screen.getByText('Error message')).toBeTruthy()
    expect(screen.getByText('✕')).toBeTruthy()
  })

  it('displays warning notification', async () => {
    const user = userEvent.setup()

    render(
      <TestWrapper>
        <NotificationTrigger />
      </TestWrapper>
    )

    await user.click(screen.getByText('Trigger Warning'))

    expect(screen.getByText('Warning message')).toBeTruthy()
    expect(screen.getByText('⚠')).toBeTruthy()
  })

  it('displays info notification', async () => {
    const user = userEvent.setup()

    render(
      <TestWrapper>
        <NotificationTrigger />
      </TestWrapper>
    )

    await user.click(screen.getByText('Trigger Info'))

    expect(screen.getByText('Info message')).toBeTruthy()
    expect(screen.getByText('ℹ')).toBeTruthy()
  })

  it('displays multiple notifications', async () => {
    const user = userEvent.setup()

    render(
      <TestWrapper>
        <NotificationTrigger />
      </TestWrapper>
    )

    await user.click(screen.getByText('Trigger Success'))
    await user.click(screen.getByText('Trigger Error'))

    expect(screen.getByText('Success message')).toBeTruthy()
    expect(screen.getByText('Error message')).toBeTruthy()
  })

  it('removes notification when close button clicked', async () => {
    const user = userEvent.setup()

    render(
      <TestWrapper>
        <NotificationTrigger />
      </TestWrapper>
    )

    await user.click(screen.getByText('Trigger Success'))
    expect(screen.getByText('Success message')).toBeTruthy()

    const closeButton = screen.getByLabelText('Dismiss notification')
    await user.click(closeButton)

    expect(screen.queryByText('Success message')).toBeNull()
  })

  it('removes notification when close button is clicked', async () => {
    const user = userEvent.setup()

    render(
      <TestWrapper>
        <NotificationTrigger />
      </TestWrapper>
    )

    await user.click(screen.getByText('Trigger Success'))
    expect(screen.getByText('Success message')).toBeTruthy()

    const closeButton = screen.getByLabelText('Dismiss notification')
    await user.click(closeButton)

    expect(screen.queryByText('Success message')).toBeNull()
  })

  it('applies correct CSS class for notification type', async () => {
    const user = userEvent.setup()

    render(
      <TestWrapper>
        <NotificationTrigger />
      </TestWrapper>
    )

    await user.click(screen.getByText('Trigger Success'))

    const notification = screen.getByText('Success message').closest('.notification')
    expect(notification?.classList.contains('notification-success')).toBe(true)
  })
})
