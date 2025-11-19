import { ReactNode } from 'react'

interface LayoutProps {
  children: ReactNode
  currentView: string
  onNavigate: (view: string) => void
}

export function Layout({ children, currentView, onNavigate }: LayoutProps) {
  return (
    <div className="app-container">
      <aside className="sidebar">
        <div className="sidebar-header">
          <h2>CreatorOps</h2>
        </div>
        <nav className="sidebar-nav">
          <NavItem
            icon="📊"
            label="Dashboard"
            active={currentView === 'dashboard'}
            onClick={() => onNavigate('dashboard')}
          />
          <NavItem
            icon="💿"
            label="Import"
            active={currentView === 'import'}
            onClick={() => onNavigate('import')}
          />
          <NavItem
            icon="📁"
            label="Projects"
            active={currentView === 'projects'}
            onClick={() => onNavigate('projects')}
          />
          <NavItem
            icon="💾"
            label="Backup Queue"
            active={currentView === 'backup'}
            onClick={() => onNavigate('backup')}
          />
          <NavItem
            icon="⚙️"
            label="Settings"
            active={currentView === 'settings'}
            onClick={() => onNavigate('settings')}
          />
        </nav>
      </aside>
      <main className="main-content">{children}</main>
    </div>
  )
}

interface NavItemProps {
  icon: string
  label: string
  active?: boolean
  onClick: () => void
}

function NavItem({ icon, label, active = false, onClick }: NavItemProps) {
  return (
    <div className={`nav-item ${active ? 'active' : ''}`} onClick={onClick}>
      <span role="img" aria-label={label}>
        {icon}
      </span>
      <span>{label}</span>
    </div>
  )
}
