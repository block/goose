import { Info } from "lucide-react";
import { Tooltip, TooltipTrigger, TooltipPortal, TooltipContent } from "@/components/ui/tooltip";

export default function DescriptionTooltip({ description, className, icon = <Info className={`h-3 w-3 cursor-pointer ${className}`} /> }: { description: string, className?: string, icon?: React.ReactNode }) {
	return (
		<Tooltip delayDuration={0}>
			<TooltipTrigger asChild>
				{icon ? icon : <Info className={`h-3 w-3 cursor-pointer ${className}`} />}
			</TooltipTrigger>
			<TooltipPortal>
				<TooltipContent className="max-w-xs whitespace-pre-wrap">{description}</TooltipContent>
			</TooltipPortal>
		</Tooltip>
	);
};