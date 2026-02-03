import {
	Breadcrumb,
	BreadcrumbList,
	BreadcrumbItem,
	BreadcrumbLink,
	BreadcrumbSeparator,
	BreadcrumbPage,
} from "@/components/ui/breadcrumb";
import { usePageHeader } from "@/selectors/page";
import { usePathname } from "next/navigation";
import React, { useEffect } from "react";
import { generatePageHeader } from "@/utils/breadcrumbs";

export default function RouteBreadcrumbs() {
	const pathname = usePathname();
	const { header, setHeader } = usePageHeader();

	useEffect(() => {
		const pageHeader = generatePageHeader(pathname);
		setHeader(pageHeader);
	}, [pathname, setHeader]);

	const isHomePage = pathname === "/home";
	const showHomeBreadcrumb = !isHomePage;
	const hasIntermediateBreadcrumbs = header.breadcrumbs.length > 0;

	return (
		<Breadcrumb className="grow px-2 py-2 rounded-none">
			<BreadcrumbList className="text-xs">
				{showHomeBreadcrumb && (
					<BreadcrumbItem>
						<BreadcrumbLink href="/home">
							Home
						</BreadcrumbLink>
					</BreadcrumbItem>
				)}
				
				{showHomeBreadcrumb && hasIntermediateBreadcrumbs && <BreadcrumbSeparator />}
				
				{header.breadcrumbs.map(({ title, href }, index) => (
					<React.Fragment key={href}>
						<BreadcrumbItem>
							<BreadcrumbLink href={href}>
								{title}
							</BreadcrumbLink>
						</BreadcrumbItem>
						{index < header.breadcrumbs.length - 1 && <BreadcrumbSeparator />}
					</React.Fragment>
				))}
				
				{(showHomeBreadcrumb || hasIntermediateBreadcrumbs) && <BreadcrumbSeparator />}
				
				<BreadcrumbItem>
					<BreadcrumbPage className="capitalize">
						{header.title}
					</BreadcrumbPage>
				</BreadcrumbItem>
			</BreadcrumbList>
		</Breadcrumb>
	);
}
