import {
	EvalutatedResponseData,
	ProviderType,
	Providers,
} from "@/types/store/openground";
import { ProviderResult } from "@/lib/platform/openground-clickhouse";
import {
	Table,
	TableBody,
	TableHead,
	TableHeader,
	TableRow,
} from "@/components/ui/table";
import { useRootStore } from "@/store";
import { removeProvider } from "@/selectors/openground";
import { Button } from "@/components/ui/button";
import { FlaskRoundIcon, Trash2Icon } from "lucide-react";
import { JsonViewer } from "@textea/json-viewer";
import { omit } from "lodash";
import Image from "next/image";
import getMessage from "@/constants/messages";

const keyHeadersTransformer = (str: string) => {
	// Step 1: Capitalize the first letter
	str = str.charAt(0).toUpperCase() + str.slice(1);

	// Step 2: Insert a space before any uppercase letter that follows a lowercase letter
	str = str.replace(/([a-z])([A-Z])/g, "$1 $2");

	// Step 3: Convert any uppercase letters after the first character to lowercase
	str = str.charAt(0) + str.slice(1).toLowerCase();

	return str;
};

const dataAdditionalStrings: Record<
	string,
	{ prefix?: string; suffix?: string }
> = {
	cost: {
		prefix: "$",
	},
	responseTime: {
		suffix: "s",
	},
};

const priorityDisplayOrder = ["prompt", "response"];

export default function ProviderTable({
	provider,
	index,
	selectedProviders,
	evaluatedResponse,
}: {
	provider: ProviderType;
	index: number;
	evaluatedResponse: {
		isLoading: boolean;
		data?: EvalutatedResponseData | ProviderResult[];
	};
	selectedProviders: {
		provider: Providers;
		config: Record<string, any>;
	}[];
}) {
	const removeProviderItem = useRootStore(removeProvider);
	const onClickDelete = () => removeProviderItem(index);
	const selectedProvider = selectedProviders[index];

	// This component only works with the legacy data format (tuple format)
	// Cast to legacy type for backward compatibility
	const legacyData = evaluatedResponse.data as EvalutatedResponseData | undefined;

	const keysDisplayOrder = [
		...priorityDisplayOrder,
		...Object.keys(
			legacyData?.[index]?.[1]?.evaluationData || {}
		).filter((k) => !priorityDisplayOrder.includes(k)),
	];
	return (
		<Table className="h-full relative">
			<TableHeader className="bg-stone-300 dark:bg-stone-700 text-stone-600 dark:text-stone-200 border-none z-20 sticky top-0">
				<TableRow>
					<TableHead colSpan={2}>
						<div className="flex w-full gap-3">
							<span className="grow">
								{provider.title} ({provider.subTitle})
							</span>
							{!(evaluatedResponse.data || evaluatedResponse.isLoading) && (
								<Button
									variant="ghost"
									size="icon"
									className="rounded-full shrink-0 h-auto w-auto"
									onClick={onClickDelete}
								>
									<Trash2Icon className="h-4 w-4" />
									<span className="sr-only">{getMessage().DELETE}</span>
								</Button>
							)}
						</div>
					</TableHead>
				</TableRow>
			</TableHeader>
			<TableBody className="text-stone-900 dark:text-stone-300">
				{legacyData ? (
					legacyData?.[index]?.[1] ? (
						<>
							{keysDisplayOrder.map((key) => (
								<TableRow key={key}>
									<section className="flex flex-col w-full h-full relative">
										<p className="font-medium p-4 bg-stone-200 dark:bg-stone-800 text-stone-500 sticky top-[48px]">
											{keyHeadersTransformer(key)}
										</p>
										<p
											className={`text-stone-600 dark:text-stone-400 p-4 ${
												key === "response"
													? "h-[200px] overflow-auto"
													: key === "prompt"
													? "max-h-[200px] overflow-auto"
													: ""
											}`}
										>
											{dataAdditionalStrings[key]?.prefix || ""}
											{legacyData?.[index]?.[1].evaluationData[key]}
											{dataAdditionalStrings[key]?.suffix || ""}
										</p>
									</section>
								</TableRow>
							))}
							<TableRow>
								<section className="flex flex-col w-full h-full relative">
									<p className="font-medium p-4 bg-stone-200 dark:bg-stone-800 text-stone-500 sticky top-[48px]">
										{getMessage().OPENGROUND_PROVIDER_RESPONSE}
									</p>
									<JsonViewer
										value={omit(legacyData?.[index]?.[1], [
											"evaluationData",
										])}
										className="overflow-auto p-3 h-[400px] !rounded-none"
										enableClipboard={false}
										displayDataTypes={false}
										displaySize={false}
										theme="dark"
									/>
								</section>
							</TableRow>
						</>
					) : (
						<TableRow>
							<div className="flex flex-col w-full h-full items-center justify-center p-4 text-error text-center">
								{legacyData?.[index]?.[0] ||
									getMessage().SOME_ERROR_OCCURRED}
							</div>
						</TableRow>
					)
				) : (
					<TableRow>
						<section className="flex flex-col w-full h-full items-center justify-center p-4 group">
							{evaluatedResponse.isLoading ? (
								<FlaskRoundIcon className="w-32 h-32 text-stone-200 dark:text-stone-700" />
							) : (
								<>
									<Image
										src={provider.logoDark}
										width={200}
										height={40}
										alt={provider.title}
										className="dark:hidden opacity-50 group-hover:opacity-100"
									/>
									<Image
										src={provider.logo}
										width={200}
										height={40}
										alt={provider.title}
										className="hidden dark:block opacity-50 group-hover:opacity-100"
									/>
								</>
							)}
						</section>
					</TableRow>
				)}
			</TableBody>
		</Table>
	);
}
