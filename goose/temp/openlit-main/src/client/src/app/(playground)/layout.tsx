import Sidebar from "@/components/(playground)/sidebar";
import Header from "@/components/(playground)/header";
import { Suspense } from "react";
import ClickhouseConnectivityWrapper from "@/components/(playground)/clickhouse-connectivity-wrapper";
import { TooltipProvider } from "@/components/ui/tooltip";
import CustomPostHogProvider from "@/components/(playground)/posthog";
import NavigationEvents from "@/components/common/navigation-events";
import AppInit from "@/components/common/app-init";
import { PortalProvider } from "@/components/(playground)/header-portal";

export default async function PlaygroundLayout({
	children,
}: {
	children: React.ReactNode;
}) {
	const telemetryEnabled = process.env.TELEMETRY_ENABLED !== "false";

	return (
		<CustomPostHogProvider telemetryEnabled={telemetryEnabled}>
			<TooltipProvider>
				<PortalProvider>
					<div className="flex h-screen w-full gap-4 overflow-hidden p-2">
						<Sidebar />
						<div className="flex flex-col grow w-full">
							<Header />
							<main className="flex flex-col grow flex-1 items-start p-0 overflow-hidden">
								<ClickhouseConnectivityWrapper>
									{children}
								</ClickhouseConnectivityWrapper>
							</main>
						</div>
					</div>
				</PortalProvider>
			</TooltipProvider>
			<Suspense fallback={null}>
				<NavigationEvents />
				<AppInit />
			</Suspense>
		</CustomPostHogProvider>
	);
}
