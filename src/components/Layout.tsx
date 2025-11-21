import { ReactNode } from 'react'
import dashboardIcon from '../assets/icons/dashboard.png'
import dashboardIconSelected from '../assets/icons/dashboard_selected.png'
import importIcon from '../assets/icons/import.png'
import importIconSelected from '../assets/icons/import_selected.png'
import projectsIcon from '../assets/icons/dir.png'
import projectsIconSelected from '../assets/icons/dir_selected.png'
import backupIcon from '../assets/icons/archive.png'
import backupIconSelected from '../assets/icons/archive_selected.png'
import deliveryIcon from '../assets/icons/delivery.png'
import deliveryIconSelected from '../assets/icons/delivery_selected.png'
import historyIcon from '../assets/icons/history.png'
import historyIconSelected from '../assets/icons/history_selected.png'
import settingsIcon from '../assets/icons/settings.png'
import settingsIconSelected from '../assets/icons/settings_selected.png'

interface LayoutProps {
  children: ReactNode
  currentView: string
  onNavigate: (view: string) => void
  importCount?: number
  projectsCount?: number
}

export function Layout({
  children,
  currentView,
  onNavigate,
  importCount,
  projectsCount,
}: LayoutProps) {
  return (
    <div className="app-container">
      <aside className="sidebar">
        <div className="sidebar-header">
          <h2>CreatorOps</h2>
        </div>
        <nav className="sidebar-nav">
          <NavItem
            iconSrc={dashboardIcon}
            iconSrcSelected={dashboardIconSelected}
            label="Dashboard"
            active={currentView === 'dashboard'}
            onClick={() => onNavigate('dashboard')}
          />
          <NavItem
            iconSrc={importIcon}
            iconSrcSelected={importIconSelected}
            label="Import"
            active={currentView === 'import'}
            count={importCount}
            onClick={() => onNavigate('import')}
          />
          <NavItem
            iconSrc={projectsIcon}
            iconSrcSelected={projectsIconSelected}
            label="Projects"
            active={currentView === 'projects'}
            count={projectsCount}
            onClick={() => onNavigate('projects')}
          />
          <NavItem
            iconSrc={backupIcon}
            iconSrcSelected={backupIconSelected}
            label="Backup Queue"
            active={currentView === 'backup'}
            onClick={() => onNavigate('backup')}
          />
          <NavItem
            iconSrc={deliveryIcon}
            iconSrcSelected={deliveryIconSelected}
            label="Delivery"
            active={currentView === 'delivery'}
            onClick={() => onNavigate('delivery')}
          />
          <NavItem
            iconSrc={historyIcon}
            iconSrcSelected={historyIconSelected}
            label="History"
            active={currentView === 'history'}
            onClick={() => onNavigate('history')}
          />
          <NavItem
            iconSrc={settingsIcon}
            iconSrcSelected={settingsIconSelected}
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
  icon?: string
  iconSrc?: string
  iconSrcSelected?: string
  label: string
  active?: boolean
  count?: number
  onClick: () => void
}

function NavItem({ icon, iconSrc, iconSrcSelected, label, active = false, count, onClick }: NavItemProps) {
  const currentIcon = active && iconSrcSelected ? iconSrcSelected : iconSrc

  return (
    <div className={`nav-item ${active ? 'active' : ''}`} onClick={onClick}>
      {currentIcon ? (
        <img src={currentIcon} alt={label} className="nav-item-icon" />
      ) : (
        <span role="img" aria-label={label}>
          {icon}
        </span>
      )}
      <span>{label}</span>
      {count !== undefined && count > 0 && <span className="nav-item-badge">{count}</span>}
    </div>
  )
}
