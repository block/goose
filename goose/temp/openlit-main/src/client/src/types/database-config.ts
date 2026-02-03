import { DatabaseConfig as PrismaDatabaseConfig } from "@prisma/client";
import { MouseEventHandler } from "react";

export interface DatabaseConfig extends PrismaDatabaseConfig {}

export type DatabaseConfigTabItemProps = {
	id: string;
	name: string;
	badge?: string;
	isCurrent?: boolean;
	canDelete?: boolean;
	canShare?: boolean;
	canEdit?: boolean;
};

export type DatabaseConfigTabsProps = {
	items: DatabaseConfigTabItemProps[];
	onClickTab: MouseEventHandler<HTMLElement>;
	selectedTabId: string;
	onClickItemDelete?: MouseEventHandler<SVGSVGElement>;
	onClickItemChangeActive: MouseEventHandler<HTMLDivElement>;
	addButton?: boolean;
};
