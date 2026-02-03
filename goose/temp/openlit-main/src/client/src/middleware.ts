import { chain } from "@/middleware/chain";
import checkAuth from "@/middleware/check-auth";
import checkDemoAccount from "@/middleware/check-demo-account";

export const middleware = chain([
	checkDemoAccount,
	// @ts-expect-error Type 'NextMiddlewareWithAuth' is not assignable to type 'NextMiddleware'.
	checkAuth,
]);

export const config = {
	matcher: [
		"/api/:path*",
		"/login",
		"/register",
		"/getting-started",
		"/dashboard",
		"/requests",
		"/openground",
		"/exceptions",
		"/prompt-hub",
		"/vault",
		"/settings",
		"/telemetry-enabled",
		"/home",
		"/dashboards/:path*",
		"/d/:path*",
		"/fleet-hub"
	],
};
