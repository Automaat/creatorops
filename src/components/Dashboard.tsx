export function Dashboard() {
  return (
    <>
      <div className="content-header">
        <h1>Dashboard</h1>
        <p className="text-secondary">Overview of your photography workflow</p>
      </div>
      <div className="content-body">
        <div className="flex flex-col gap-xl">
          <section>
            <h2>Active Projects</h2>
            <div className="flex flex-col gap-md">
              <div className="card">
                <p className="text-secondary">No active projects</p>
              </div>
            </div>
          </section>

          <section>
            <h2>Recent Imports</h2>
            <div className="flex flex-col gap-md">
              <div className="card">
                <p className="text-secondary">No recent imports</p>
              </div>
            </div>
          </section>

          <section>
            <h2>Storage Statistics</h2>
            <div className="card">
              <p className="text-secondary">Storage stats coming soon</p>
            </div>
          </section>
        </div>
      </div>
    </>
  )
}
