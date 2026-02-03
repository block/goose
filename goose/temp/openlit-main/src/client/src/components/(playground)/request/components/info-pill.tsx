import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Separator } from "@/components/ui/separator";

export default function InfoPill({
	title,
	value,
}: {
	title: string;
	value: any;
}) {
	return (
		<Button
			variant="outline"
			size="default"
			className="text-stone-500 bg-stone-300 dark:text-stone-300 dark:bg-stone-800 cursor-default px-2 py-1 h-auto overflow-hidden"
		>
			<span className="text-xs bg-transparent">{title}</span>
			<Separator
				orientation="vertical"
				className="mx-1 h-4 bg-stone-300 dark:bg-stone-600"
			/>
			<Badge
				variant="secondary"
				className="rounded-sm px-1 font-normal bg-transparent py-0 block ellipsis overflow-hidden whitespace-normal"
			>
				{value}
			</Badge>
		</Button>
	);
}
