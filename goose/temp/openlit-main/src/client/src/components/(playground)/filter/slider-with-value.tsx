import { FilterConfig } from "@/types/store/filter";
import { Label } from "@/components/ui/label";
import { Slider } from "@/components/ui/slider";
import { Input } from "@/components/ui/input";
import { ChangeEventHandler } from "react";

export default function SlideWithValue({
	label,
	value,
	maxValue,
	onChange,
	step,
	type,
}: {
	label: string;
	value: number;
	maxValue: number;
	onChange: (type: keyof FilterConfig, value: any) => void;
	step?: number;
	type: keyof FilterConfig;
}) {
	const onSliderValueChange = (changedValue: number[]) => {
		onChange(type, (changedValue[0] * maxValue) / 100);
	};

	const onInputValueChange: ChangeEventHandler<HTMLInputElement> = (ev) => {
		onChange(type, parseFloat(ev.target.value));
	};

	const percentageValue = (value / (maxValue || 1)) * 100;
	const stepValue = step || maxValue / 1000;

	return (
		<div className="flex items-center shrink-0 w-96 pl-3 pr-2 border border-stone-200 dark:border-0 dark:bg-stone-800 rounded-md text-stone-500 gap-2">
			<div className="flex items-center justify-between shrink-0">
				<Label htmlFor={type} className="text-xs">{label}</Label>
			</div>
			<Slider
				id={type}
				max={100}
				defaultValue={[percentageValue]}
				value={[percentageValue]}
				step={stepValue}
				onValueChange={onSliderValueChange}
				className="[&_[role=slider]]:h-4 [&_[role=slider]]:w-4 w-36 ml-3 shrink-0"
				aria-label={label}
			/>
			<Input
				step={stepValue}
				value={value}
				onChange={onInputValueChange}
				className="border-0 text-right p-0 bg-transparent dark:bg-transparent h-auto"
				type="number"
			/>
		</div>
	);
}
