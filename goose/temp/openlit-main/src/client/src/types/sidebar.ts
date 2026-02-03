import { ReactElement } from "react";

export type SidebarSection = {
	title: string;
	type: "section";
	collapsible?: boolean;
	className?: string;
	children?: SidebarActionItem[];
}

export type SidebarActionItem = {
	// className?: string;
	component?: ReactElement;
	icon?: ReactElement;
	text: string;
	link?: string;
	onClick?: any;
	target?: string;
	type: "action";
}

export type SidebarItemProps = SidebarActionItem | SidebarSection;