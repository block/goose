"use client";
import Image from "next/image";
import { usePathname } from "next/navigation";
import Link from "next/link";
import {
	Tooltip,
	TooltipContent,
	TooltipTrigger,
} from "@/components/ui/tooltip";
import { Button, buttonVariants } from "@/components/ui/button";
import { SidebarActionItem, SidebarItemProps } from "@/types/sidebar";
import { SIDEBAR_ITEMS } from "@/constants/sidebar";
import { PanelRightClose, PanelRightOpen } from "lucide-react";
import { useState } from "react";
import version from "../../../../package.json";
import UserActions from "./user-actions";
import { Accordion, AccordionTrigger, AccordionItem, AccordionContent } from "@/components/ui/accordion";
import { PRIMARY_BACKGROUND } from "@/constants/common-classes";

const getIfSidebarItemClasses = (pathname: string, item: SidebarItemProps) => {
	const commonClasses = "flex gap-2 group-data-[state=open]:w-auto group-data-[state=open]:justify-start p-2.5 font-normal ";
	const activeClasses = "text-white bg-primary dark:bg-primary dark:text-white [&]:hover:bg-primary [&]:dark:hover:bg-primary hover:text-white [&>.external-icon-svg]:fill-white [&>.external-icon-svg]:dark:fill-white ";
	const inactiveClasses = "text-stone-500 dark:text-stone-300 hover:bg-stone-700 dark:hover:bg-stone-600 hover:text-white [&>.external-icon-svg]:fill-stone-500 [&>.external-icon-svg]:dark:fill-stone-300 [&>.external-icon-svg]:hover:fill-white ";

	if (item.type === "section")
		return `${inactiveClasses}${commonClasses}`;

	if(item.link) {
		if (["/home", "/dashboard", "/requests", "/exceptions", "/prompt-hub", "/vault", "/openground", "/settings"].includes(item.link)) {
			return pathname.startsWith(item.link) ? `${activeClasses}${commonClasses}` : `${inactiveClasses}${commonClasses}`;
		}

		if (["/dashboards"].includes(item.link)) {
			return pathname.startsWith("/dashboards") || pathname.startsWith("/d/") ? `${activeClasses}${commonClasses}` : `${inactiveClasses}${commonClasses}`;
		}

		if (["/settings/api-keys"].includes(item.link)) {
			return pathname.startsWith(item.link) ? `${activeClasses}${commonClasses}` : `${inactiveClasses}${commonClasses}`;
		}

		return pathname.startsWith(item.link) ? `${activeClasses}${commonClasses}` : `${inactiveClasses}${commonClasses}`;
	}

	switch (item.link) {
		case "/home":
			return pathname.startsWith("/home") ? `${activeClasses}${commonClasses}` : `${inactiveClasses}${commonClasses}`;
		case "/dashboards":
			return pathname.startsWith("/dashboards") || pathname.startsWith("/d/") ? `${activeClasses}${commonClasses}` : `${inactiveClasses}${commonClasses}`;
		case "/dashboard":
			return pathname.startsWith("/dashboard") && !pathname.startsWith("/dashboards") ? `${activeClasses}${commonClasses}` : `${inactiveClasses}${commonClasses}`;
		case "/requests":
			return pathname.startsWith("/requests") ? `${activeClasses}${commonClasses}` : `${inactiveClasses}${commonClasses}`;
		case "/exceptions":
			return pathname.startsWith("/exceptions") ? `${activeClasses}${commonClasses}` : `${inactiveClasses}${commonClasses}`;
		case "/prompt-hub":
			return pathname.startsWith("/prompt-hub") ? `${activeClasses}${commonClasses}` : `${inactiveClasses}${commonClasses}`;
		case "/vault":
			return pathname.startsWith("/vault") ? `${activeClasses}${commonClasses}` : `${inactiveClasses}${commonClasses}`;
		case "/openground":
			return pathname.startsWith("/openground") ? `${activeClasses}${commonClasses}` : `${inactiveClasses}${commonClasses}`;
		case "/settings":
			return pathname.startsWith("/settings") ? `${activeClasses}${commonClasses}` : `${inactiveClasses}${commonClasses}`;
		default:
			return `${inactiveClasses}${commonClasses}`;
	}
};

const SidebarActionItemComponent = ({ item, className }: { item: SidebarActionItem, className?: string }) => {
	if (item.component) return item.component;

	return (
		<Tooltip delayDuration={0}>
			<TooltipTrigger asChild>
				{!item.target && item.link ? (
					<Link
						className={`${buttonVariants({
							variant: "ghost",
							size: "icon",
						})} ${className || ""}`}
						href={item.link}
						aria-label={item.text}
					>
						{item.icon}
						<span className="group-data-[state=open]:block hidden">{item.text}</span>
					</Link>
				) : (
					<a
						href={item.link}
						className={`flex items-center p-2 ${buttonVariants({
							variant: "ghost",
							size: "icon",
						})} ${className || ""
							}`}
						onClick={item.onClick}
						target={item.target}
					>
						{item.icon}
						<span className="group-data-[state=open]:block hidden">{item.text}</span>
					</a>
				)}
			</TooltipTrigger>
			<TooltipContent side="right" sideOffset={5} className="group-data-[state=open]:hidden">
				{item.text}
			</TooltipContent>
		</Tooltip>
	);
}

const SidebarItem = ({ item, className, pathname }: { item: SidebarItemProps, className?: string, pathname: string }) => {
	if (item.type === "section") {
		return (
			<Accordion
				type="single"
				className="w-full"
				collapsible={item.collapsible}
				defaultValue={item.title}
			>
				<AccordionItem value={item.title} className="border-0">
					<AccordionTrigger className="py-1 hover:no-underline  [&>svg]:text-stone-400 [&>svg]:dark:text-stone-500 [&[aria-disabled=true]>svg]:hidden group-data-[state=close]:[&[aria-disabled=true]_hr]:block [&[aria-disabled=true]_p:first-of-type]:hidden">
						<div className="flex items-center h-5 grow">
							<Tooltip delayDuration={0}>
								<TooltipTrigger asChild className="hidden group-data-[state=close]:flex grow pl-2">
									<p className="text-xs text-stone-400 dark:text-stone-500 uppercase">{item.title.substring(0, 1)}</p>
								</TooltipTrigger>
								<TooltipContent side="right" sideOffset={5} className="group-data-[state=open]:hidden">
									{item.title}
								</TooltipContent>
							</Tooltip>
							<hr className="hidden border-stone-200 dark:border-stone-800 border-t w-full" />
							<p className="text-xs text-stone-400 dark:text-stone-500 text-center group-data-[state=open]:block hidden shrink-0 px-2">{item.title}</p>
						</div>
					</AccordionTrigger>
					<AccordionContent className="flex flex-col pb-0">
						{item.children?.map((child, index) => (
							<SidebarActionItemComponent key={`sidebar-${item.title}-${index}`} item={child} className={getIfSidebarItemClasses(pathname, child)} />
						))}
					</AccordionContent>
				</AccordionItem>
			</Accordion>
		);
	}

	return <SidebarActionItemComponent item={item} className={className} />
};

export default function Sidebar() {
	const pathname = usePathname();
	const [isExpanded, setIsExpanded] = useState<boolean>(true);

	const toggleExpansion = () => setIsExpanded(e => !e);

	return (
		<aside
			aria-label="Sidebar"
			data-state={isExpanded ? "open" : "close"}
			className={`inset-y flex h-full flex-col relative group shrink-0 gap-[1px] border border-stone-200 dark:border-stone-800 ${PRIMARY_BACKGROUND} rounded-md ${isExpanded ? "w-60" : "w-auto"}`}
		>
			<div className="flex border-b border-stone-200 dark:border-stone-800 p-2 relative items-center">
				<Image
					className="size-10 flex-shrink-0 transition duration-75 p-1"
					src="/images/logo.png"
					alt="openlit's Logo"
					priority
					width={24}
					height={24}
				/>
				<p className="flex gap-1 items-center text-lg font-semibold capitalize text-primary group-data-[state=close]:hidden">
					<span>OpenLIT</span>
					<span className="text-xs font-normal text-primary/80 mt-1">({version.version})</span>
				</p>
				<Button variant="ghost" size="icon" aria-label="Expand" onClick={toggleExpansion} className="absolute top-1/2 right-0 -translate-y-1/2 translate-x-1/2 rounded-md h-auto w-auto text-stone-500 dark:text-stone-300 p-0.5 cursor-pointer bg-stone-50 hover:bg-stone-50 dark:bg-stone-900 dark:hover:bg-stone-900">
					{isExpanded ? <PanelRightOpen className="w-5 h-5" /> : <PanelRightClose className="w-5 h-5" />}
				</Button>
			</div>
			<nav className="flex flex-col p-2 pt-4 overflow-auto grow scrollbar-hidden">
				{SIDEBAR_ITEMS.map((item, index) => (
					<SidebarItem
						key={`sidebar-top-${index}`}
						className={getIfSidebarItemClasses(pathname, item)}
						item={item}
						pathname={pathname}
					/>
				))}
			</nav>
			<div className="flex flex-col shrink-0 p-2">
				<UserActions />
			</div>
		</aside>
	);
}
