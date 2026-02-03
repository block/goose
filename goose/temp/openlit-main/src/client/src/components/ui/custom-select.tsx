"use client";

import * as React from "react";
import { Check, ChevronsUpDown } from "lucide-react";

import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import {
	Command,
	CommandEmpty,
	CommandGroup,
	CommandInput,
	CommandItem,
	CommandList,
} from "@/components/ui/command";
import {
	Popover,
	PopoverContent,
	PopoverTrigger,
} from "@/components/ui/popover";
import { Input } from "@/components/ui/input";
import { CustomSelectProps } from "@/types/form";

export function CustomSelect({
	options,
	defaultValue = "",
	onChange = () => {},
	placeholder = "Select an option...",
	hasOtherOption,
	name,
	id,
}: CustomSelectProps) {
	const [open, setOpen] = React.useState(false);
	const [value, setValue] = React.useState(defaultValue);
	const [inputValue, setInputValue] = React.useState(defaultValue);
	const [popoverWidth, setPopoverWidth] = React.useState(0);
	const [showOtherInput, setShowOtherInput] = React.useState(false);
	const triggerRef = React.useRef<HTMLButtonElement>(null);
	const popoverRef = React.useRef<HTMLDivElement>(null);

	const updatePopoverWidth = React.useCallback(() => {
		if (triggerRef.current) {
			const width = triggerRef.current.offsetWidth;
			setPopoverWidth(width);
		}
	}, []);

	React.useEffect(() => {
		updatePopoverWidth();
		window.addEventListener("resize", updatePopoverWidth);
		return () => window.removeEventListener("resize", updatePopoverWidth);
	}, [updatePopoverWidth]);

	React.useEffect(() => {
		if (defaultValue) {
			const option = options.find((opt) => opt.value === defaultValue);
			if (option) {
				setValue(defaultValue);
				setInputValue(defaultValue);
				setShowOtherInput(false);
			} else {
				setValue("other");
				setInputValue(defaultValue);
				setShowOtherInput(true);
			}
			onChange(defaultValue);
		}
	}, [defaultValue, options, onChange]);

	const handleSelect = (currentValue: string) => {
		if (currentValue === "other") {
			setValue("other");
			setInputValue("");
			setShowOtherInput(true);
			onChange("");
		} else {
			setValue(currentValue);
			setInputValue(currentValue);
			setShowOtherInput(false);
			onChange(currentValue);
		}
		setOpen(false);
	};

	const handleInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
		const newValue = e.target.value;
		setInputValue(newValue);
		onChange(newValue);
	};

	return (
		<div className="space-y-2">
			<Popover open={open} onOpenChange={setOpen}>
				<PopoverTrigger asChild>
					<Button
						variant="outline"
						role="combobox"
						aria-expanded={open}
						className="w-full justify-between"
						ref={triggerRef}
					>
						{value === "other" && inputValue
							? inputValue
							: options.find((option) => option.value === value)?.label ||
							  placeholder}
						<ChevronsUpDown className="ml-2 h-4 w-4 shrink-0 opacity-50" />
					</Button>
				</PopoverTrigger>
				<PopoverContent
					ref={popoverRef}
					className="w-[var(--radix-popover-trigger-width)] p-0"
					style={
						{
							"--radix-popover-trigger-width": `${popoverWidth}px`,
						} as React.CSSProperties
					}
					onOpenAutoFocus={(e) => e.preventDefault()}
				>
					<Command>
						<CommandInput placeholder="Search option..." />
						<CommandList>
							<CommandEmpty>No option found.</CommandEmpty>
							<CommandGroup>
								{options.map((option) => (
									<CommandItem
										key={option.value}
										value={option.value}
										onSelect={handleSelect}
									>
										<Check
											className={cn(
												"mr-2 h-4 w-4",
												value === option.value ? "opacity-100" : "opacity-0"
											)}
										/>
										{option.label}
									</CommandItem>
								))}
								{hasOtherOption && (
									<CommandItem value="other" onSelect={handleSelect}>
										<Check
											className={cn(
												"mr-2 h-4 w-4",
												value === "other" ? "opacity-100" : "opacity-0"
											)}
										/>
										Other
									</CommandItem>
								)}
							</CommandGroup>
						</CommandList>
					</Command>
				</PopoverContent>
			</Popover>
			<div
				className={cn(
					"transition-all duration-200 ease-in-out",
					showOtherInput ? "opacity-100" : "opacity-0 h-0 overflow-hidden"
				)}
			>
				<Input
					type="text"
					placeholder="Enter other option"
					value={inputValue}
					onChange={handleInputChange}
					name={name}
					id={id}
				/>
			</div>
		</div>
	);
}
