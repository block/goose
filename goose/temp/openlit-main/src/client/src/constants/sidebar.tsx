import { SidebarItemProps } from "@/types/sidebar";
import {
	BookKey,
	BookText,
	Component,
	Home,
	Key,
	LayoutDashboard,
	MonitorCog,
	MonitorPlay,
	ShieldAlert,
	TextQuote
} from "lucide-react";
import DatabaseConfigSwitch from "@/components/(playground)/sidebar/database-config-switch";
import OpenTelemetrySvg from "@/components/svg/opentelemetry";

export const ICON_CLASSES = "flex-shrink-0 size-5";

export const SIDEBAR_ITEMS: SidebarItemProps[] = [
	{
		icon: <Home className={ICON_CLASSES} />,
		text: "Home",
		link: "/home",
		type: "action",
	},
	{
		icon: <LayoutDashboard className={ICON_CLASSES} />,
		text: "Dashboards",
		link: "/dashboards",
		type: "action",
	},
	{
		title: "Monitoring",
		type: "section",
		children: [{
			icon: <TextQuote className={ICON_CLASSES} />,
			text: "Requests",
			link: "/requests",
			type: "action",
		},
		{
			icon: <ShieldAlert className={ICON_CLASSES} />,
			text: "Exceptions",
			link: "/exceptions",
			type: "action",
		},
		{
			icon: <OpenTelemetrySvg className={ICON_CLASSES} />,
			text: "Fleet Hub",
			link: "/fleet-hub",
			type: "action",
		},
		]
	},
	{
		title: "Resources",
		type: "section",
		children: [{
			icon: <Component className={ICON_CLASSES} />,
			text: "Prompt Hub",
			link: "/prompt-hub",
			type: "action",
		},
		{
			icon: <BookKey className={ICON_CLASSES} />,
			text: "Vault",
			link: "/vault",
			type: "action",
		},
		{
			icon: <MonitorPlay className={ICON_CLASSES} />,
			text: "Openground",
			link: "/openground",
			type: "action",
		},
		]
	},
	{
		title: "Configuration",
		type: "section",
		children: [{
			text: "Database Switch",
			component: <DatabaseConfigSwitch />,
			type: "action",
		},
		{
			icon: <MonitorCog className={ICON_CLASSES} />,
			text: "Evaluation Config",
			link: "/settings/evaluation",
			type: "action",
		},
		]
	},
	{
		title: "Settings",
		type: "section",
		collapsible: true,
		children: [
			{
				icon: <Key className={ICON_CLASSES} />,
				text: "Api Keys",
				link: "/settings/api-keys",
				type: "action",
			},
		]
	},
	{
		text: "Documentation",
		link: "https://docs.openlit.io/",
		target: "_blank",
		icon: <BookText className={ICON_CLASSES} />,
		type: "action",
	}
];