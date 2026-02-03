import getMessage from "@/constants/messages";
import { PageHeader } from "@/types/store/page";

export interface BreadcrumbConfig {
	title: string;
	href: string;
}

export interface RouteConfig {
	regex: RegExp;
	getTitle: (pathname: string, params?: Record<string, string>) => string;
	getBreadcrumbs: (pathname: string, params?: Record<string, string>) => BreadcrumbConfig[];
	getDescription?: (pathname: string, params?: Record<string, string>) => string;
}

// Utility function to extract route parameters from pathname
export function extractRouteParams(pathname: string, regex: RegExp): Record<string, string> {
	const match = pathname.match(regex);
	if (!match) return {};
	
	const params: Record<string, string> = {};
	// Extract UUID from dashboard routes
	if (regex.source.includes('\\[0-9a-f\\]')) {
		const uuidMatch = pathname.match(/[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}/);
		if (uuidMatch) {
			params.id = uuidMatch[0];
		}
	}
	
	// Extract prompt hub ID
	if (pathname.includes('/prompt-hub/') && !pathname.endsWith('/prompt-hub')) {
		const parts = pathname.split('/');
		const idIndex = parts.indexOf('prompt-hub') + 1;
		if (idIndex < parts.length) {
			params.id = parts[idIndex];
		}
	}
	
	// Extract vault ID
	if (pathname.includes('/vault/') && !pathname.endsWith('/vault')) {
		const parts = pathname.split('/');
		const idIndex = parts.indexOf('vault') + 1;
		if (idIndex < parts.length) {
			params.id = parts[idIndex];
		}
	}
	
	// Extract openground ID
	if (pathname.includes('/openground/') && !pathname.endsWith('/openground')) {
		const parts = pathname.split('/');
		const idIndex = parts.indexOf('openground') + 1;
		if (idIndex < parts.length) {
			params.id = parts[idIndex];
		}
	}
	
	return params;
}

// Route configurations for all pages
export const ROUTE_CONFIGS: RouteConfig[] = [
	// Home page
	{
		regex: /^\/home$/,
		getTitle: () => "Home",
		getBreadcrumbs: () => [],
	},
	
	// Dashboard routes
	{
		regex: /^\/dashboard$/,
		getTitle: () => "Dashboard",
		getBreadcrumbs: () => [],
	},
	
	// Individual dashboard view
	{
		regex: /^\/d\/[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/,
		getTitle: () => "Dashboard",
		getBreadcrumbs: () => [
			{ title: "Dashboards", href: "/dashboards" }
		],
	},
	
	// Dashboard management
	{
		regex: /^\/dashboards$/,
		getTitle: () => "Dashboards",
		getBreadcrumbs: () => [],
	},
	
	{
		regex: /^\/dashboards\/board$/,
		getTitle: () => "Board",
		getBreadcrumbs: () => [
			{ title: "Dashboards", href: "/dashboards" }
		],
	},
	
	{
		regex: /^\/dashboards\/explorer$/,
		getTitle: () => "Explorer",
		getBreadcrumbs: () => [
			{ title: "Dashboards", href: "/dashboards" }
		],
	},
	
	{
		regex: /^\/dashboards\/widget$/,
		getTitle: () => "Widget",
		getBreadcrumbs: () => [
			{ title: "Dashboards", href: "/dashboards" }
		],
	},
	
	// Requests
	{
		regex: /^\/requests$/,
		getTitle: () => "Requests",
		getBreadcrumbs: () => [],
	},
	
	// Exceptions
	{
		regex: /^\/exceptions$/,
		getTitle: () => "Exceptions",
		getBreadcrumbs: () => [],
	},
	
	// Prompt Hub
	{
		regex: /^\/prompt-hub$/,
		getTitle: () => "Prompt Hub",
		getBreadcrumbs: () => [],
	},
	
	{
		regex: /^\/prompt-hub\/[^/]+$/,
		getTitle: (pathname, params) => {
			// In a real app, you might want to fetch the prompt name
			return params?.id ? "Prompt Details" : "Prompt";
		},
		getBreadcrumbs: () => [
			{ title: "Prompt Hub", href: "/prompt-hub" }
		],
	},
	
	// Vault
	{
		regex: /^\/vault$/,
		getTitle: () => "Vault",
		getBreadcrumbs: () => [],
	},
	
	{
		regex: /^\/vault\/[^/]+$/,
		getTitle: () => "Vault Item",
		getBreadcrumbs: () => [
			{ title: "Vault", href: "/vault" }
		],
	},

	// Openground
	{
		regex: /^\/openground$/,
		getTitle: () => "Openground",
		getBreadcrumbs: () => [],
	},
	
	{
		regex: /^\/openground\/[^/]+$/,
		getTitle: (pathname, params) => {
			if (pathname === "/openground/new") return getMessage().OPENGROUND_CREATE_NEW_PLAYGROUND;
			if (pathname === "/openground/models") return getMessage().OPENGROUND_MANAGE_MODELS;
			return params?.id ? getMessage().OPENGROUND_RUN_DETAILS : getMessage().FEATURE_OPENGROUND;
		},
		getBreadcrumbs: () => [
			{ title: getMessage().FEATURE_OPENGROUND, href: "/openground" }
		],
	},
	
	// Settings
	{
		regex: /^\/settings$/,
		getTitle: () => "Settings",
		getBreadcrumbs: () => [],
	},
	
	{
		regex: /^\/settings\/profile$/,
		getTitle: () => "User Profile",
		getBreadcrumbs: () => [
			{ title: "Settings", href: "/settings" }
		],
	},
	
	{
		regex: /^\/settings\/evaluation$/,
		getTitle: () => "Evaluation Settings",
		getBreadcrumbs: () => [
			{ title: "Settings", href: "/settings" }
		],
	},
	
	{
		regex: /^\/settings\/database-config$/,
		getTitle: () => "Database Config",
		getBreadcrumbs: () => [
			{ title: "Settings", href: "/settings" }
		],
	},
	
	{
		regex: /^\/settings\/api-keys$/,
		getTitle: () => "API Keys",
		getBreadcrumbs: () => [
			{ title: "Settings", href: "/settings" }
		],
	},
	
	// Fleet Hub
	{
		regex: /^\/fleet-hub\/[^/]+$/,
		getTitle: () => "Fleet Hub",
		getBreadcrumbs: () => [
			{ title: "Fleet Hub", href: "/fleet-hub" }
		],
	},
];

// Main function to generate page header based on pathname
export function generatePageHeader(pathname: string): PageHeader {
	const config = ROUTE_CONFIGS.find(({ regex }) => regex.test(pathname));
	
	if (!config) {
		// Fallback for unknown routes
		const pathParts = pathname.split('/').filter(Boolean);
		const title = pathParts[pathParts.length - 1]?.replace(/-/g, ' ') || 'Page';
		
		return {
			title: title.charAt(0).toUpperCase() + title.slice(1),
			breadcrumbs: [],
		};
	}
	
	const params = extractRouteParams(pathname, config.regex);
	
	return {
		title: config.getTitle(pathname, params),
		description: config.getDescription?.(pathname, params),
		breadcrumbs: config.getBreadcrumbs(pathname, params),
	};
}

// Hook for dynamic breadcrumb updates (for cases where you need to update title/description after data loads)
export function updatePageHeaderWithData(
	baseHeader: PageHeader,
	data: { title?: string; description?: string }
): PageHeader {
	return {
		...baseHeader,
		title: data.title || baseHeader.title,
		description: data.description || baseHeader.description,
	};
}
