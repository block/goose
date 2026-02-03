import { CheckIcon, PlusCircleIcon } from "lucide-react";

import { cn } from "@/lib/utils";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
	Command,
	CommandEmpty,
	CommandGroup,
	CommandInput,
	CommandItem,
	CommandList,
	CommandSeparator,
} from "@/components/ui/command";
import {
	Popover,
	PopoverContent,
	PopoverTrigger,
} from "@/components/ui/popover";
import { Separator } from "@/components/ui/separator";

interface ComboDropdownProps {
	title?: string;
	options: {
		label: string;
		value: string;
		icon?: React.ComponentType<{ className?: string }>;
	}[];
	selectedValues?: any[];
	type: any;
	clearItem?: (type: any) => void;
	updateSelectedValues: (type: any, value: any, operationType?: any) => void;
}

export default function ComboDropdown({
	title,
	options,
	selectedValues,
	type,
	clearItem,
	updateSelectedValues,
}: ComboDropdownProps) {
	const resetFilter = () => clearItem && clearItem(type);

	return (
		<Popover>
			<PopoverTrigger asChild>
				<Button
					variant="outline"
					size="default"
					className="text-stone-500 hover:text-stone-600 dark:text-stone-400 dark:hover:text-stone-300 dark:bg-stone-800 dark:hover:bg-stone-900 h-auto py-1 text-xs"
				>
					<PlusCircleIcon className="mr-2 h-3 w-3" />
					{title}
					{selectedValues?.length ? (
						<>
							<Separator
								orientation="vertical"
								className="mx-2 h-3 bg-stone-300 dark:bg-stone-600"
							/>
							<Badge
								variant="secondary"
								className="rounded-sm px-1 font-normal lg:hidden"
							>
								{selectedValues.length}
							</Badge>
							<div className="hidden space-x-1 lg:flex">
								{selectedValues.length > 2 ? (
									<Badge
										variant="secondary"
										className="rounded-sm px-1 font-normal py-0"
									>
										{selectedValues.length} selected
									</Badge>
								) : (
									options
										.filter((option) => selectedValues.includes(option.value))
										.map((option) => (
											<Badge
												variant="secondary"
												key={option.value}
												className="rounded-sm px-1 py-0 font-normal capitalize max-w-[100px] text-ellipsis overflow-hidden inline-block"
											>
												{option.label}
											</Badge>
										))
								)}
							</div>
						</>
					) : null}
				</Button>
			</PopoverTrigger>
			<PopoverContent className="w-[200px] p-0 cursor-pointer" align="start">
				<Command>
					<CommandInput placeholder={title} />
					<CommandList>
						<CommandEmpty>No results found.</CommandEmpty>
						<CommandGroup>
							{options.map((option) => {
								const isSelected = selectedValues?.includes(option.value);
								return (
									<CommandItem
										className="cursor-pointer capitalize"
										key={option.value}
										onSelect={() => {
											if (isSelected) {
												updateSelectedValues(type, option.value, "delete");
											} else {
												updateSelectedValues(type, option.value, "add");
											}
										}}
									>
										<div
											className={cn(
												"mr-2 flex h-4 w-4 items-center justify-center rounded-sm border border-primary",
												isSelected
													? "bg-primary text-primary-foreground"
													: "opacity-50 [&_svg]:invisible"
											)}
										>
											<CheckIcon className={"h-4 w-4 text-stone-100"} />
										</div>
										{option.icon && (
											<option.icon className="mr-2 h-4 w-4 text-muted-foreground" />
										)}
										<span>{option.label}</span>
									</CommandItem>
								);
							})}
						</CommandGroup>
						{selectedValues?.length ? (
							<>
								<CommandSeparator />
								<CommandGroup>
									<CommandItem
										onSelect={resetFilter}
										className="justify-center text-center cursor-pointer"
									>
										Reset
									</CommandItem>
								</CommandGroup>
							</>
						) : null}
					</CommandList>
				</Command>
			</PopoverContent>
		</Popover>
	);
}
