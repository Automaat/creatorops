import { useTheme } from '../hooks/useTheme'

export function Settings() {
  const { theme, setTheme } = useTheme()

  return (
    <>
      <div className="content-header">
        <h1>Settings</h1>
        <p className="text-secondary">Configure CreatorOps preferences</p>
      </div>
      <div className="content-body">
        <div className="flex flex-col gap-xl">
          <section>
            <h2>Appearance</h2>
            <div className="card">
              <div className="flex flex-col gap-md">
                <div>
                  <label className="font-medium">Theme</label>
                  <p className="text-secondary text-sm" style={{ marginBottom: 'var(--space-sm)' }}>
                    Choose how CreatorOps looks
                  </p>
                </div>
                <div className="flex gap-md">
                  <button
                    className={`btn ${theme === 'system' ? 'btn-primary' : 'btn-secondary'}`}
                    onClick={() => setTheme('system')}
                  >
                    System
                  </button>
                  <button
                    className={`btn ${theme === 'light' ? 'btn-primary' : 'btn-secondary'}`}
                    onClick={() => setTheme('light')}
                  >
                    Light
                  </button>
                  <button
                    className={`btn ${theme === 'dark' ? 'btn-primary' : 'btn-secondary'}`}
                    onClick={() => setTheme('dark')}
                  >
                    Dark
                  </button>
                </div>
              </div>
            </div>
          </section>

          <section>
            <h2>Storage Paths</h2>
            <div className="card">
              <div className="flex flex-col gap-md">
                <div className="flex flex-col gap-xs">
                  <label className="font-medium">Default Import Location</label>
                  <p className="text-secondary text-sm">~/CreatorOps/Projects</p>
                </div>
                <div className="flex flex-col gap-xs">
                  <label className="font-medium">Backup Destinations</label>
                  <p className="text-secondary text-sm">Not configured</p>
                </div>
                <div className="flex flex-col gap-xs">
                  <label className="font-medium">Archive Location</label>
                  <p className="text-secondary text-sm">Not configured</p>
                </div>
              </div>
            </div>
          </section>

          <section>
            <h2>Import Settings</h2>
            <div className="card">
              <div className="flex flex-col gap-md">
                <div className="flex flex-col gap-xs">
                  <label className="font-medium">Auto-eject SD cards after import</label>
                  <p className="text-secondary text-sm">Coming soon</p>
                </div>
                <div className="flex flex-col gap-xs">
                  <label className="font-medium">File renaming rules</label>
                  <p className="text-secondary text-sm">Keep original names</p>
                </div>
              </div>
            </div>
          </section>
        </div>
      </div>
    </>
  )
}
