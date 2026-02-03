import {
	Dialog,
	DialogContent,
	DialogHeader,
	DialogTitle,
	DialogDescription,
	DialogFooter,
	DialogClose,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Widget, WidgetType } from "../types";
import { useState } from "react";
import { SUPPORTED_WIDGETS } from "../constants";

export default function WidgetSelectionModal({
	open,
	onClose,
	widgets,
	onSelect,
	onCreateNew,
}: {
	open: boolean;
	onClose: () => void;
	widgets: Widget[];
	onSelect: (widget: Widget) => void;
	onCreateNew: (widgetType: WidgetType) => void;
}) {
	const [createNewWidget, setCreateNewWidget] = useState<boolean>(false);

	const handleCreateNew = () => {
		setCreateNewWidget(true);
	};

	const handleWidgetSelect = (widgetType: WidgetType) => {
		setCreateNewWidget(false);
		onCreateNew(widgetType);
	};

	return (
		<Dialog open={open} onOpenChange={(v) => !v && onClose()}>
			<DialogContent>
				<DialogHeader>
					<DialogTitle>Select a Widget</DialogTitle>
					<DialogDescription>
						Choose an existing widget to add, or create a new one.
					</DialogDescription>
				</DialogHeader>
				<div className="max-h-64 overflow-y-auto my-4">
					{
						createNewWidget ? (
							<div className="grid grid-cols-2 gap-2 text-center text-stone-500">
								{
									Object.entries(SUPPORTED_WIDGETS).map(([type, widget]) => (
										<Button key={type} variant="outline" className="w-full justify-start" onClick={() => handleWidgetSelect(type as WidgetType)}>
											<div className="flex items-center gap-2">
												<widget.icon className="h-4 w-4" />
												<span className="font-semibold mr-2">{widget.name}</span>
											</div>
										</Button>
									))
								}
							</div>
						) : (
							widgets.length === 0 ? (
								<div className="text-center text-stone-500">No widgets found.</div>
							) : (
								<ul className="space-y-2">
									{widgets.map((w) => (
										<li key={w.id}>
											<Button
												variant="outline"
												className="w-full justify-start"
												onClick={() => onSelect(w)}
											>
												<span className="font-semibold mr-2">{w.title}</span>
												<span className="text-xs text-stone-500">({w.type})</span>
											</Button>
										</li>
									))}
								</ul>
							)
						)
					}
				</div>
				<DialogFooter className="flex justify-between gap-2">
					{
						createNewWidget ? (
							<Button onClick={() => setCreateNewWidget(false)} variant="default">
								Back to Widgets
							</Button>
						) : (
							<Button onClick={handleCreateNew} variant="default">
								Create New Widget
							</Button>
						)
					}
					<DialogClose asChild>
						<Button variant="ghost" className="bg-stone-200 hover:bg-stone-300 dark:bg-stone-800 dark:hover:bg-stone-700 dark:text-white">Cancel</Button>
					</DialogClose>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	);
}
