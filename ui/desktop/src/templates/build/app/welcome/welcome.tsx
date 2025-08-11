export function Welcome() {
  return (
    <main className="min-h-screen gradient-bg dark:gradient-bg-light">
      <div className="container mx-auto px-6">
        <div className="max-w-5xl mx-auto text-center">
          {/* Hero Section */}
          <div className="h-[80vh] flex flex-col justify-center items-center mb-32">
            <div className="flex flex-col items-center justify-center text-center">
              {/* Logo/Icon */}
              <div className="mb-8">
                <div className="inline-flex items-center justify-center w-20 h-20 bg-black dark:bg-black rounded-xl shadow-lg">
                  <svg
                    className="w-10 h-10 text-white dark:text-white"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                  >
                    <path
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      strokeWidth={2}
                      d="M13 10V3L4 14h7v7l9-11h-7z"
                    />
                  </svg>
                </div>
              </div>

              {/* Main Heading */}
              <h1 className="text-6xl md:text-7xl font-light text-black dark:text-black mb-8 leading-tight">
                Welcome to Your New
                <br />
                <span className="text-gray-600 dark:text-gray-600 relative inline-block">
                  <span className="shimmer-text">Anything</span>
                </span>
              </h1>

              {/* Subtitle */}
              <p className="text-2xl text-gray-600 dark:text-gray-600 mb-12 max-w-3xl leading-relaxed">
                Built with Goose - your AI-powered development assistant. This is just
                the beginning of something amazing.
              </p>

              {/* CTA Button */}
              <div className="flex flex-col items-center gap-4">
                <button className="bg-black dark:bg-black text-white dark:text-white hover:bg-gray-800 dark:hover:bg-gray-800 px-8 py-3 rounded-full font-normal text-base transition-all duration-200 shadow-lg hover:shadow-xl">
                  Get Building →
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>

      {/* Feature Section */}
      <div className="mb-24">
        <div>
          {/* Feature Section Header */}
          <div className="max-w-5xl mx-auto pt-12 pb-6 px-12 text-center">
            <h2 className="text-4xl font-light text-black dark:text-black mb-8">
              Build Faster with AI
            </h2>
            <p className="text-xl text-gray-600 dark:text-gray-600 mb-12 max-w-3xl mx-auto leading-relaxed">
              Goose helps you create, modify, and deploy websites using natural language.
              No coding experience required.
            </p>
          </div>
          <div className="max-w-5xl mx-auto py-6 px-12 text-center">
            <div className="flex justify-center mb-6">
              <div className="w-12 h-12 bg-black dark:bg-black rounded-lg flex items-center justify-center shadow-lg">
                <svg
                  className="w-6 h-6 text-white dark:text-white"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M13 10V3L4 14h7v7l9-11h-7z"
                  />
                </svg>
              </div>
            </div>
            <div>
              <h3 className="text-3xl font-light text-black dark:text-black mb-4">
                Lightning Fast
              </h3>
              <p className="text-gray-600 dark:text-gray-600 text-xl leading-relaxed max-w-2xl mx-auto">
                Generate and modify code instantly with AI assistance
              </p>
            </div>
          </div>
        </div>

        <div>
          <div className="max-w-5xl mx-auto py-12 px-12 text-center">
            <div className="flex justify-center mb-6">
              <div className="w-12 h-12 bg-black dark:bg-black rounded-lg flex items-center justify-center shadow-lg">
                <svg
                  className="w-6 h-6 text-white dark:text-white"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"
                  />
                </svg>
              </div>
            </div>
            <div>
              <h3 className="text-3xl font-light text-black dark:text-black mb-4">
                Smart & Reliable
              </h3>
              <p className="text-gray-600 dark:text-gray-600 text-xl leading-relaxed max-w-2xl mx-auto">
                Built with modern frameworks and best practices
              </p>
            </div>
          </div>
        </div>

        <div>
          <div className="max-w-5xl mx-auto py-12 px-12 text-center">
            <div className="flex justify-center mb-6">
              <div className="w-12 h-12 bg-black dark:bg-black rounded-lg flex items-center justify-center shadow-lg">
                <svg
                  className="w-6 h-6 text-white dark:text-white"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M4.318 6.318a4.5 4.5 0 000 6.364L12 20.364l7.682-7.682a4.5 4.5 0 00-6.364-6.364L12 7.636l-1.318-1.318a4.5 4.5 0 00-6.364 0z"
                  />
                </svg>
              </div>
            </div>
            <div>
              <h3 className="text-3xl font-light text-black dark:text-black mb-4">
                Easy to Use
              </h3>
              <p className="text-gray-600 dark:text-gray-600 text-xl leading-relaxed max-w-2xl mx-auto">
                Natural language commands make development accessible
              </p>
            </div>
          </div>
        </div>
      </div>
    </main>
  );
}
