import { DashboardItemType } from "@/types/manage-dashboard";
import { useCallback, useState } from "react";

export type AddResource = {
	title: string;
	description: string;
	type: DashboardItemType;
	tags: string[];
	parentPath: string[] | null;
}

export type EditResource = {
	itemId: string,
	newTitle: string,
	newDescription: string,
	newTags: string[];
	parentPath: string[] | null;
}

type DialogState = {
	isOpen: boolean;
	mode: "add" | "edit";
	itemTitle: string;
	itemDescription: string;
	itemType: DashboardItemType;
	itemTags: string[];
	currentPath: string[] | null;
	editingItemId: string | null;
};

export const useUpsertResource = ({
	addItem,
	editItem,
}: {
	addItem?: (item: AddResource) => void;
	editItem: (item: EditResource) => void;
}) => {
	const [dialogState, setDialogState] = useState<DialogState>({
		isOpen: false,
		mode: "add" as "add" | "edit",
		itemTitle: "",
		itemDescription: "",
		itemTags: [],
		itemType: "board" as DashboardItemType,
		currentPath: [] as string[],
		editingItemId: null as string | null,
	});

	const handleDialogSave = useCallback(
		(title: string, description: string, type: DashboardItemType, tags: string[]) => {
			const { mode, currentPath, editingItemId } = dialogState;

			if (mode === "add") {
				addItem?.({
					title,
					description,
					type,
					tags,
					parentPath: currentPath,
				});
			} else if (editingItemId) {
				editItem({
					itemId: editingItemId,
					newTitle: title,
					newDescription: description,
					newTags: tags,
					parentPath: currentPath,
				});
			}
		},
		[addItem, dialogState, editItem]
	);

	const handleDialogCancel = useCallback(() => {
		setDialogState((prev) => ({
			...prev,
			isOpen: false,
		}));
	}, []);

	return {
		dialogState,
		setDialogState,
		handleDialogSave,
		handleDialogCancel,
	};
};