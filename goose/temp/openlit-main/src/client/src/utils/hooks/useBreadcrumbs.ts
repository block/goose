import { useEffect } from "react";
import { usePathname } from "next/navigation";
import { usePageHeader } from "@/selectors/page";
import { generatePageHeader, updatePageHeaderWithData } from "@/utils/breadcrumbs";
import { PageHeader } from "@/types/store/page";

/**
 * Hook to automatically manage breadcrumbs based on the current route
 * This hook will automatically set the page header when the pathname changes
 */
export function useBreadcrumbs() {
	const pathname = usePathname();
	const { header, setHeader } = usePageHeader();

	useEffect(() => {
		const pageHeader = generatePageHeader(pathname);
		setHeader(pageHeader);
	}, [pathname, setHeader]);

	return { header, setHeader };
}

/**
 * Hook to manage breadcrumbs with dynamic data
 * Use this when you need to update the page title or description after data loads
 * 
 * @param data - Object containing title and/or description to override defaults
 * @param deps - Dependencies array for when to update the breadcrumbs
 */
export function useDynamicBreadcrumbs(
	data: { title?: string; description?: string },
	deps: React.DependencyList = []
) {
	const pathname = usePathname();
	const { header, setHeader } = usePageHeader();

	useEffect(() => {
		const baseHeader = generatePageHeader(pathname);
		const updatedHeader = updatePageHeaderWithData(baseHeader, data);
		setHeader(updatedHeader);
	}, [pathname, setHeader, data.title, data.description, ...deps]);

	return { header, setHeader };
}

/**
 * Hook to manually set custom breadcrumbs
 * Use this for complex cases where the automatic breadcrumbs don't work
 * 
 * @param customHeader - Custom page header to set
 * @param deps - Dependencies array for when to update the breadcrumbs
 */
export function useCustomBreadcrumbs(
	customHeader: PageHeader,
	deps: React.DependencyList = []
) {
	const { header, setHeader } = usePageHeader();

	useEffect(() => {
		setHeader(customHeader);
	}, [setHeader, customHeader.title, customHeader.description, ...deps]);

	return { header, setHeader };
}
