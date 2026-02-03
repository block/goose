"use client";
import { usePathname, useRouter } from "next/navigation";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import type React from "react";
import ThemeToggleSwitch from "@/components/(playground)/sidebar/theme-switch";
import { PRIMARY_BACKGROUND } from "@/constants/common-classes";

export default function SettingsLayout({
	children,
}: {
	children: React.ReactNode;
}) {
	const router = useRouter();
	const pathname = usePathname();

	const tabs = [
		{
			value: "evaluation",
			label: "Evaluation Settings",
			path: "/settings/evaluation",
		},
		{ value: "profile", label: "User Profile", path: "/settings/profile" },
		{
			value: "database",
			label: "Database Config",
			path: "/settings/database-config",
		},
		{ value: "api-keys", label: "API Keys", path: "/settings/api-keys" },
	];

	const currentTab =
		tabs.find((tab) => pathname.startsWith(tab.path))?.value || "profile";

	const handleTabChange = (value: string) => {
		const tab = tabs.find((t) => t.value === value);
		if (tab) {
			router.push(tab.path);
		}
	};

	return (

		<Tabs
			value={currentTab}
			onValueChange={handleTabChange}
			className="w-full flex flex-col h-full"
		>
			<div className="flex items-center justify-between gap-4 mb-4">
				<TabsList className="p-0 h-[30px] grid grid-cols-4 self-start border border-stone-200 dark:border-stone-800">
					{tabs.map((tab) => (
						<TabsTrigger key={tab.value} value={tab.value} className="py-1.5 text-xs rounded-md">
							{tab.label}
						</TabsTrigger>
					))}
				</TabsList>
				<ThemeToggleSwitch />
			</div>
			<div className={`flex w-full border border-stone-200 dark:border-stone-800 ${PRIMARY_BACKGROUND} rounded-lg grow overflow-hidden`}>
				{children}
			</div>
		</Tabs>
	);
}
