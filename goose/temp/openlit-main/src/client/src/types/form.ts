import { InputProps } from "@/components/ui/input";
import { TextareaProps } from "@/components/ui/textarea";
import type { FormEvent, ReactNode } from "react";
import { SwitchProps } from "@/components/ui/switch";
export interface Option {
	value: string;
	label: string;
}

export interface CustomSelectProps {
	options: Option[];
	defaultValue?: string;
	onChange: (value: string) => void;
	placeholder?: string;
	hasOtherOption?: boolean;
	name: string;
	id?: string;
}

export type FieldTypes =
	| "INPUT"
	| "TEXTAREA"
	| "TAGSINPUT"
	| "KEYVALUE"
	| "RADIOGROUP"
	| "SELECT"
	| "SWITCH";

export interface RadioGroupProps {
	name: string;
	options: {
		title: string;
		subText: string;
		description?: string;
		value: string;
	}[];
	defaultValue?: string;
	placeholder?: string;
	onKeyUp?: any;
	onChange?: any;
	id?: string
}

export interface KeyValuePairProps {
	name: string;
	defaultValue?: { key: string; value: any }[];
	placeholder?: string;
	onKeyUp?: any;
	onChange?: any;
	id?: string;
}

export type FieldTypeProps =
	| InputProps
	| TextareaProps
	| RadioGroupProps
	| KeyValuePairProps
	| CustomSelectProps
	| SwitchProps;

export type FieldProps = {
	label: string;
	description?: string | ReactNode;
	fieldType: FieldTypes;
	fieldTypeProps: FieldTypeProps;
	inputKey?: string;
	id?: string;
};

export type FormBuilderEvent = (e: FormEvent<Element>, formData: any) => void;
