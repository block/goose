// Admin Dashboard Components (exe.dev - Client-Side Only)
// Injected into unified-exe.html by assemble-sell-exe.js
// Uses CONFIG object and CSS variables for theming
// NOTE: No backend API calls - uses Clerk metadata as source of truth

// === Subscription Badge Component ===
function SubscriptionBadge({ status }) {
  const badges = {
    active: { bg: 'bg-green-500/20', text: 'text-green-600', label: 'Subscribed' },
    trial: { bg: 'bg-yellow-500/20', text: 'text-yellow-600', label: 'Trial' },
    past_due: { bg: 'bg-red-500/20', text: 'text-red-600', label: 'Past Due' },
    canceled: { bg: 'bg-gray-500/20', text: 'text-gray-600', label: 'Canceled' },
  };
  const badge = badges[status] || { bg: 'bg-blue-500/20', text: 'text-blue-600', label: 'Free' };

  return (
    <span className={`px-2 py-1 rounded text-xs font-bold ${badge.bg} ${badge.text}`}>
      {badge.label}
    </span>
  );
}

// === Admin Dashboard (Client-Side Only) ===
function AdminDashboard() {
  const [activeTab, setActiveTab] = useState('overview');
  const { user } = useUser();

  // SECURITY: Do NOT add fallbacks like "|| CONFIG.adminUserIds.length === 0"
  // Empty admin list means NO admin access, not everyone is admin
  const isAdmin = CONFIG.adminUserIds.includes(user?.id);

  if (!isAdmin) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-[var(--admin-bg)] p-4">
        <div className="max-w-md text-center p-8 bg-[var(--admin-card-bg)] border-4 border-[var(--admin-border)] shadow-[8px_8px_0px_var(--admin-shadow)]">
          <h2 className="text-2xl font-bold mb-4 text-[var(--admin-text)]">Access Denied</h2>
          <p className="text-[var(--admin-text-muted)] mb-4">You don't have admin permissions.</p>
          <p className="text-sm text-[var(--admin-text-muted)]">Your user ID: {user?.id}</p>
        </div>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-[var(--admin-bg)] p-4">
      <div className="max-w-6xl mx-auto">
        <header className="flex justify-between items-center mb-8">
          <h1 className="text-3xl font-bold text-[var(--admin-text)]">{CONFIG.appTitle} Admin</h1>
          <UserButton />
        </header>

        {/* Client-Side Only Notice */}
        <div className="mb-6 p-4 bg-blue-100 border-4 border-blue-500 text-blue-700">
          <p className="font-bold mb-1">Client-Side Admin Dashboard</p>
          <p className="text-sm">
            This admin dashboard runs without a backend server. Tenant data is stored in Clerk user metadata.
            For full analytics, use Clerk's dashboard.
          </p>
        </div>

        {/* Tab Navigation */}
        <div className="flex gap-2 mb-6">
          {['overview', 'config'].map((tab) => (
            <button
              key={tab}
              onClick={() => setActiveTab(tab)}
              className={`px-4 py-2 font-bold border-4 border-[var(--admin-border)] transition-all capitalize ${
                activeTab === tab
                  ? 'bg-[var(--admin-border)] text-[var(--admin-card-bg)] shadow-none'
                  : 'bg-[var(--admin-card-bg)] text-[var(--admin-text)] shadow-[4px_4px_0px_var(--admin-shadow)] hover:shadow-[2px_2px_0px_var(--admin-shadow)] hover:translate-x-[2px] hover:translate-y-[2px]'
              }`}
            >
              {tab}
            </button>
          ))}
        </div>

        {/* Overview Tab */}
        {activeTab === 'overview' && (
          <>
            <div className="grid md:grid-cols-3 gap-6 mb-8">
              <div className="p-6 bg-[var(--admin-card-bg)] border-4 border-[var(--admin-border)] shadow-[6px_6px_0px_var(--admin-shadow)]">
                <h3 className="text-sm font-bold text-[var(--admin-text-muted)] mb-2">DEPLOYMENT</h3>
                <p className="text-2xl font-bold text-[var(--admin-text)]">exe.dev</p>
                <p className="text-sm text-[var(--admin-text-muted)] mt-1">Client-side only</p>
              </div>
              <div className="p-6 bg-[var(--admin-card-bg)] border-4 border-[var(--admin-border)] shadow-[6px_6px_0px_#3b82f6]">
                <h3 className="text-sm font-bold text-[var(--admin-text-muted)] mb-2">DOMAIN</h3>
                <p className="text-2xl font-bold text-blue-600 break-all">{CONFIG.domain}</p>
              </div>
              <div className="p-6 bg-[var(--admin-card-bg)] border-4 border-[var(--admin-border)] shadow-[6px_6px_0px_var(--admin-accent)]">
                <h3 className="text-sm font-bold text-[var(--admin-text-muted)] mb-2">AUTH PROVIDER</h3>
                <p className="text-2xl font-bold text-green-600">Clerk</p>
                <p className="text-sm text-[var(--admin-text-muted)] mt-1">With Billing</p>
              </div>
            </div>

            {/* Quick Links */}
            <div className="grid md:grid-cols-2 gap-6">
              <div className="bg-[var(--admin-card-bg)] border-4 border-[var(--admin-border)] shadow-[6px_6px_0px_var(--admin-shadow)] p-6">
                <h3 className="font-bold mb-4 text-[var(--admin-text)]">Manage Users</h3>
                <p className="text-sm text-[var(--admin-text-muted)] mb-4">
                  View and manage all users, subscriptions, and tenant data in the Clerk dashboard.
                </p>
                <a
                  href="https://dashboard.clerk.com"
                  target="_blank"
                  rel="noopener noreferrer"
                  className="inline-block px-4 py-2 bg-[var(--admin-border)] text-[var(--admin-card-bg)] font-bold border-4 border-[var(--admin-border)] hover:opacity-90 transition-opacity"
                >
                  Open Clerk Dashboard →
                </a>
              </div>
              <div className="bg-[var(--admin-card-bg)] border-4 border-[var(--admin-border)] shadow-[6px_6px_0px_var(--admin-shadow)] p-6">
                <h3 className="font-bold mb-4 text-[var(--admin-text)]">Billing & Revenue</h3>
                <p className="text-sm text-[var(--admin-text-muted)] mb-4">
                  Configure pricing plans and view revenue analytics in Clerk Billing.
                </p>
                <a
                  href="https://dashboard.clerk.com/last-active?path=billing"
                  target="_blank"
                  rel="noopener noreferrer"
                  className="inline-block px-4 py-2 bg-[var(--admin-border)] text-[var(--admin-card-bg)] font-bold border-4 border-[var(--admin-border)] hover:opacity-90 transition-opacity"
                >
                  Open Billing Dashboard →
                </a>
              </div>
            </div>

            {/* Your Admin Info */}
            <div className="mt-6 bg-[var(--admin-card-bg)] border-4 border-[var(--admin-border)] shadow-[6px_6px_0px_var(--admin-shadow)] p-6">
              <h3 className="font-bold mb-4 text-[var(--admin-text)]">Your Admin Account</h3>
              <div className="flex items-center gap-4">
                {user?.imageUrl && (
                  <img src={user.imageUrl} alt="" className="w-12 h-12 rounded-full border-2 border-[var(--admin-border)]" />
                )}
                <div>
                  <p className="font-bold text-[var(--admin-text)]">{user?.fullName || user?.firstName || 'Admin'}</p>
                  <p className="text-sm text-[var(--admin-text-muted)]">{user?.primaryEmailAddress?.emailAddress}</p>
                  <p className="text-xs text-[var(--admin-text-muted)] font-mono mt-1">{user?.id}</p>
                </div>
              </div>
            </div>
          </>
        )}

        {/* Config Tab */}
        {activeTab === 'config' && (
          <>
            {/* App Configuration */}
            <div className="bg-[var(--admin-card-bg)] border-4 border-[var(--admin-border)] shadow-[6px_6px_0px_var(--admin-shadow)] p-6 mb-6">
              <h3 className="font-bold mb-4 text-[var(--admin-text)]">App Configuration</h3>
              <div className="grid md:grid-cols-2 gap-4">
                <div className="p-4 bg-[var(--admin-bg)] border-2 border-[var(--admin-border)]">
                  <p className="text-sm font-bold text-[var(--admin-text-muted)]">App Name</p>
                  <p className="font-mono text-[var(--admin-text)]">{CONFIG.appName}</p>
                </div>
                <div className="p-4 bg-[var(--admin-bg)] border-2 border-[var(--admin-border)]">
                  <p className="text-sm font-bold text-[var(--admin-text-muted)]">App Title</p>
                  <p className="text-[var(--admin-text)]">{CONFIG.appTitle}</p>
                </div>
                <div className="p-4 bg-[var(--admin-bg)] border-2 border-[var(--admin-border)]">
                  <p className="text-sm font-bold text-[var(--admin-text-muted)]">Domain</p>
                  <p className="font-mono text-[var(--admin-text)]">{CONFIG.domain}</p>
                </div>
                <div className="p-4 bg-[var(--admin-bg)] border-2 border-[var(--admin-border)]">
                  <p className="text-sm font-bold text-[var(--admin-text-muted)]">Tagline</p>
                  <p className="text-[var(--admin-text)]">{CONFIG.tagline}</p>
                </div>
              </div>
            </div>

            {/* Billing Configuration */}
            <div className="bg-[var(--admin-card-bg)] border-4 border-[var(--admin-border)] shadow-[6px_6px_0px_var(--admin-shadow)] p-6 mb-6">
              <h3 className="font-bold mb-4 text-[var(--admin-text)]">Billing Configuration</h3>
              <div className="p-4 bg-[var(--admin-bg)] border-2 border-[var(--admin-border)]">
                <p className="text-sm font-bold text-[var(--admin-text-muted)]">Billing Mode</p>
                <p className="text-xl font-bold text-[var(--admin-text)] capitalize">{CONFIG.billingMode || 'off'}</p>
                <p className="text-sm text-[var(--admin-text-muted)] mt-2">
                  {CONFIG.billingMode === 'required'
                    ? 'Users must subscribe via Clerk Billing to access the app.'
                    : 'All users have free access after signing in.'}
                </p>
              </div>
              <p className="text-sm text-[var(--admin-text-muted)] mt-4">
                Manage subscription plans in{' '}
                <a
                  href="https://dashboard.clerk.com/last-active?path=billing"
                  target="_blank"
                  rel="noopener noreferrer"
                  className="text-blue-600 hover:underline"
                >
                  Clerk Billing Dashboard
                </a>.
              </p>
            </div>

            {/* Features List */}
            <div className="bg-[var(--admin-card-bg)] border-4 border-[var(--admin-border)] shadow-[6px_6px_0px_var(--admin-shadow)] p-6 mb-6">
              <h3 className="font-bold mb-4 text-[var(--admin-text)]">Features ({CONFIG.features.length})</h3>
              <ul className="space-y-2">
                {CONFIG.features.map((feature, i) => (
                  <li key={i} className="flex items-center gap-2 text-[var(--admin-text)]">
                    <svg className="w-4 h-4 text-green-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" />
                    </svg>
                    {feature}
                  </li>
                ))}
              </ul>
            </div>

            {/* Admin Users */}
            <div className="bg-[var(--admin-card-bg)] border-4 border-[var(--admin-border)] shadow-[6px_6px_0px_var(--admin-shadow)] p-6">
              <h3 className="font-bold mb-4 text-[var(--admin-text)]">Admin User IDs ({CONFIG.adminUserIds.length})</h3>
              {CONFIG.adminUserIds.length === 0 ? (
                <p className="text-[var(--admin-text-muted)]">No admin users configured. Add user IDs to --admin-ids during assembly.</p>
              ) : (
                <ul className="space-y-2">
                  {CONFIG.adminUserIds.map((id, i) => (
                    <li key={i} className="font-mono text-sm text-[var(--admin-text)] p-2 bg-[var(--admin-bg)] border border-[var(--admin-border)]">
                      {id}
                      {id === user?.id && <span className="ml-2 text-green-600">(you)</span>}
                    </li>
                  ))}
                </ul>
              )}
            </div>
          </>
        )}
      </div>
    </div>
  );
}

// === Admin App Wrapper ===
function AdminApp() {
  return (
    <ClerkProvider publishableKey={CONFIG.clerkPublishableKey} afterSignOutUrl={`https://${CONFIG.domain}`}>
      <SignedIn>
        <AdminDashboard />
      </SignedIn>
      <SignedOut>
        <div className="min-h-screen flex items-center justify-center bg-[var(--admin-bg)] p-4">
          <div className="max-w-md text-center p-8 bg-[var(--admin-card-bg)] border-4 border-[var(--admin-border)] shadow-[8px_8px_0px_var(--admin-shadow)]">
            <h2 className="text-2xl font-bold mb-4 text-[var(--admin-text)]">Admin Login</h2>
            <p className="mb-6 text-[var(--admin-text-muted)]">Sign in to access the admin dashboard.</p>
            <SignInButton mode="modal">
              <button className="px-6 py-3 bg-[var(--admin-border)] text-[var(--admin-card-bg)] font-bold border-4 border-[var(--admin-border)] hover:opacity-90 transition-opacity">
                Sign In
              </button>
            </SignInButton>
          </div>
        </div>
      </SignedOut>
    </ClerkProvider>
  );
}
