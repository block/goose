import {
	Fragment,
	KeyboardEvent,
	useCallback,
	useEffect,
	useRef,
	useState,
} from "react";
import { Label } from "@/components/ui/label";
import { Input, InputProps } from "@/components/ui/input";
import { Textarea, TextareaProps } from "@/components/ui/textarea";
import { PlusIcon, XIcon } from "lucide-react";
import { Button } from "@/components/ui/button";
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group";
import { CustomSelect } from "@/components/ui/custom-select";
import {
	CustomSelectProps,
	FieldProps,
	KeyValuePairProps,
	RadioGroupProps,
} from "@/types/form";
import { Switch, SwitchProps } from "@/components/ui/switch";

function FormTagsInputField(props: FieldProps) {
	const [tags, setTags] = useState<string[]>(
		(props.fieldTypeProps.defaultValue as string[]) || []
	);
	const removeTag = (tagToRemove: string) => {
		setTags(tags.filter((tag) => tag !== tagToRemove));
	};
	const addTag = useCallback(
		(e: KeyboardEvent<HTMLInputElement>) => {
			const value = e.currentTarget.value.trim();
			if (value.trim() !== "") {
				setTags((t) => {
					return Array.from(new Set([...t, value.trim()]));
				});
				e.currentTarget.value = "";
			}
		},
		[tags, setTags]
	);

	const onKeyDownHandler = (e: KeyboardEvent<HTMLInputElement>) => {
		const inputProps = props.fieldTypeProps as InputProps;
		if (typeof inputProps?.onKeyUp === "function") {
			inputProps.onKeyUp(e);
		}
		if (e.key === "Enter") {
			e.preventDefault();
			addTag(e);
		}
	};

	return (
		<div className="grid grid-col-1 gap-2">
			<Input
				className="ph-no-capture"
				key={
					props.inputKey ||
					`${props.fieldTypeProps.name}-${props.fieldTypeProps.defaultValue}`
				}
				placeholder={(props.fieldTypeProps as InputProps).placeholder}
				onKeyDown={onKeyDownHandler}
			/>
			<div className="flex flex-wrap gap-2 mb-2">
				{tags.map((tag) => (
					<Fragment key={tag}>
						<Input
							className="ph-no-capture"
							type="hidden"
							name={`${props.fieldTypeProps.name}`}
							value={tag}
						/>
						<span className="flex items-center gap-2 px-3 py-1 rounded-full text-sm flex items-center bg-stone-200 text-stone-500 dark:bg-stone-800 dark:text-stone-300 transition-none">
							{tag}
							<button onClick={() => removeTag(tag)}>
								<XIcon size={14} />
							</button>
						</span>
					</Fragment>
				))}
			</div>
		</div>
	);
}

function FormKeyValueField(props: FieldProps) {
	const [metaProperties, setMetaProperties] = useState<
		{ key: string; value: string }[]
	>(
		(props.fieldTypeProps.defaultValue as { key: string; value: string }[]) ||
			[]
	);
	const addMetaProperty = () => {
		setMetaProperties([...metaProperties, { key: "", value: "" }]);
	};

	const updateMetaProperty = (index: number, key: string, value: string) => {
		const updatedProperties = [...metaProperties];
		updatedProperties[index] = { ...updatedProperties[index], [key]: value };
		setMetaProperties(updatedProperties);
	};

	const removeMetaProperty = (index: number) => {
		setMetaProperties(metaProperties.filter((_, i) => i !== index));
	};

	const onKeyDownHandler = (e: KeyboardEvent<HTMLInputElement>) => {
		if (e.key === "Enter") {
			e.preventDefault();
		}
	};

	const { name = "keyValue", placeholder = "" } =
		props.fieldTypeProps as KeyValuePairProps;

	return (
		<div className="grid grid-col-1 w-100 gap-4">
			{metaProperties.map((prop, index) => (
				<div key={index} className="flex gap-2 items-center">
					<Input
						className="ph-no-capture"
						placeholder={`${placeholder} Key`}
						name={`${name}[${index}].key`}
						value={prop.key}
						onChange={(e) => updateMetaProperty(index, "key", e.target.value)}
						onKeyDown={onKeyDownHandler}
					/>
					<Input
						className="ph-no-capture"
						placeholder={`${placeholder} Value`}
						name={`${name}[${index}].value`}
						value={prop.value}
						onChange={(e) => updateMetaProperty(index, "value", e.target.value)}
						onKeyDown={onKeyDownHandler}
					/>
					<Button
						className="ph-no-capture"
						onClick={() => removeMetaProperty(index)}
						variant="ghost"
						size="icon"
					>
						<XIcon className="h-4 w-4" />
					</Button>
				</div>
			))}
			<Button
				onClick={addMetaProperty}
				type="button"
				variant="outline"
				className="mt-2"
			>
				<PlusIcon className="mr-2 h-4 w-4" /> Add
			</Button>
		</div>
	);
}

function FormRadioGroupField(props: FieldProps) {
	const [currentDescription, setCurrentDescription] = useState<string>("");
	const itemRef = useRef(null);
	const fieldTypeProps = props.fieldTypeProps as RadioGroupProps;
	const findIndexAndSet = (value: string) => {
		const findIndex = fieldTypeProps.options.findIndex(
			(option) => option.value === value
		);
		setCurrentDescription(fieldTypeProps.options[findIndex]?.description || "");
	};
	const onMouseEnter = (ev: any) => {
		const { value } = ev.currentTarget.dataset;
		findIndexAndSet(value);
	};

	const onMouseLeave = () => {
		let value;
		if (
			itemRef.current &&
			typeof (itemRef.current as HTMLElement).querySelector === "function"
		) {
			value = (
				(itemRef.current as HTMLElement).querySelector(
					"input:checked"
				) as HTMLInputElement
			)?.value;
		}
		value = value || (props.fieldTypeProps.defaultValue as string) || "";
		findIndexAndSet(value);
	};

	useEffect(() => {
		findIndexAndSet(props.fieldTypeProps.defaultValue as string);
	}, []);

	return (
		<>
			<RadioGroup
				onValueChange={fieldTypeProps.onChange}
				defaultValue={fieldTypeProps.defaultValue as string}
				name={fieldTypeProps.name}
				ref={itemRef}
				className={`grid grid-cols-${fieldTypeProps.options.length} gap-8 pt-2 ph-no-capture`}
			>
				{fieldTypeProps.options.map((option) => {
					return (
						<Label
							className="p-1 cursor-pointer rounded-md border dark:border-stone-800 has-[:checked]:border-stone-400 has-[:checked]:dark:border-stone-500"
							onMouseEnter={onMouseEnter}
							onMouseLeave={onMouseLeave}
							data-value={option.value}
							key={option.value}
						>
							<RadioGroupItem
								value={option.value}
								className="sr-only ph-no-capture"
							/>
							<div className="space-y-1 bg-stone-100 dark:bg-stone-800 p-4 h-full">
								<h4 className="text-sm font-semibold">{option.title}</h4>
								<p className="text-sm">{option.subText}</p>
							</div>
						</Label>
					);
				})}
			</RadioGroup>
			<div className="h-4 text-xs text-stone-400">
				{currentDescription || "Choose one version type"}
			</div>
		</>
	);
}

function FormInputField(props: FieldProps) {
	return (
		<Input
			className="col-span-2 ph-no-capture"
			key={props.inputKey || `${name}-${props.fieldTypeProps.defaultValue}`}
			{...(props.fieldTypeProps as InputProps)}
		/>
	);
}

function FormTextareaField(props: FieldProps) {
	return (
		<Textarea
			className="col-span-2 ph-no-capture"
			key={props.inputKey || `${name}-${props.fieldTypeProps.defaultValue}`}
			{...(props.fieldTypeProps as TextareaProps)}
		/>
	);
}

function FormSelectField(props: FieldProps) {
	return (
		<CustomSelect
			key={props.inputKey || `${name}-${props.fieldTypeProps.defaultValue}`}
			{...(props.fieldTypeProps as CustomSelectProps)}
		/>
	);
}

function FormSwitchField(props: FieldProps) {
	return <Switch {...(props.fieldTypeProps as SwitchProps)} />;
}

export default function FormField(
	props: FieldProps & {
		boundaryClass?: string;
	}
) {
	return (
		<div className={`${props.boundaryClass} text-stone-700 dark:text-stone-300`}>
			<Label htmlFor={props.fieldTypeProps.name} className="col-span-1">
				{props.label}
			</Label>
			{props.fieldType === "INPUT" ? (
				<FormInputField {...props} />
			) : props.fieldType === "TEXTAREA" ? (
				<FormTextareaField {...props} />
			) : props.fieldType === "TAGSINPUT" ? (
				<FormTagsInputField {...props} />
			) : props.fieldType === "KEYVALUE" ? (
				<FormKeyValueField {...props} />
			) : props.fieldType === "RADIOGROUP" ? (
				<FormRadioGroupField {...props} />
			) : props.fieldType === "SELECT" ? (
				<FormSelectField {...props} />
			) : props.fieldType === "SWITCH" ? (
				<FormSwitchField {...props} />
			) : null}
			{props.description ? (
				<span className="text-xs -mt-[5px]">
					{props.description}
				</span>
			) : null}
		</div>
	);
}
