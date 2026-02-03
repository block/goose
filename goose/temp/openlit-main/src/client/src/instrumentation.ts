/**
 * Next.js Instrumentation Hook
 * Runs before the server starts accepting requests
 * Used to run ClickHouse migrations on startup
 */

export async function register() {
	if (process.env.NEXT_RUNTIME === "nodejs") {
		console.log("🚀 Starting server initialization...");

		try {
			// Run ClickHouse migrations before server starts
			const { runClickhouseMigrations } = await import(
				"@/lib/platform/clickhouse/helpers"
			);

			console.log("📦 Running ClickHouse migrations...");
			await runClickhouseMigrations();
			console.log("✅ ClickHouse migrations completed successfully");
		} catch (error) {
			console.error("❌ Error running ClickHouse migrations on startup:", error);
			// Don't throw - allow server to start even if migrations fail
			// This prevents the server from crashing if ClickHouse is temporarily unavailable
		}

		console.log("✨ Server initialization complete");
	}
}
