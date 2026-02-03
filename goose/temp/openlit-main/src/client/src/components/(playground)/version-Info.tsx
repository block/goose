import {
	HoverCard,
	HoverCardContent,
	HoverCardTrigger,
} from "@/components/ui/hover-card";
import { ShieldQuestion } from "lucide-react";
import { Button } from "@/components/ui/button";
import version from "../../../package.json";

export default function VersionInfo() {
	return (
		<HoverCard>
			<HoverCardTrigger>
				<Button variant="ghost" size="icon">
					<ShieldQuestion className="flex-shrink-0 size-5 transition duration-75 dark:text-stone-100 text-stone-900" />
				</Button>
			</HoverCardTrigger>
			<HoverCardContent
				align="end"
				side="right"
				className="text-xs w-auto flex gap-2"
			>
				<span className="font-bold">OpenLIT: </span>
				<span>({version.version})</span>
				{process.env.NODE_ENV === "development" && (
					<span className="font-light">(Development mode)</span>
				)}
			</HoverCardContent>
		</HoverCard>
	);
}
