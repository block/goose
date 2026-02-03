const RowItemLoader = ({
	icon = false,
	text = true,
	width = "",
}: {
	icon?: boolean;
	text?: boolean;
	width?: string;
}) => (
	<div
		className={`flex ${width} flex-1 shrink-0 relative h-full justify-center items-center py-4 px-2`}
	>
		{icon && (
			<div className="h-3 w-3 mr-3 rounded-full bg-secondary/[0.9] rounded self-start shrink-0" />
		)}
		{text && (
			<div className="flex flex-col w-full justify-center space-y-3">
				<div className="h-1 w-24 bg-secondary/[0.9] rounded" />
				<div className="h-1 w-16 bg-secondary/[0.9] rounded" />
			</div>
		)}
	</div>
);

export default function RenderLoader() {
	return (
		<div className="flex flex-col mb-4 animate-pulse">
			<div className="flex items-center rounded-t py-1.5 px-3 z-0 self-start bg-secondary text-primary font-medium">
				<div className="flex items-center pr-3">
					<div className="h-3 w-3 mr-2 rounded-full bg-secondary/[0.9] rounded" />
					<div className="h-1 w-40 bg-secondary/[0.9] rounded" />
				</div>
				<div className="flex items-center pl-3 border-l border-stone-200">
					<div className="h-3 w-3 mr-2 rounded-full bg-secondary/[0.9] rounded" />
					<div className="h-1 w-14 bg-secondary/[0.9] rounded" />
				</div>
			</div>
			<div className="flex items-stretch h-16 border border-secondary relative items-center px-3 rounded-b">
				<RowItemLoader width="w-3/12" />
				<RowItemLoader icon width="w-3/12" />
				<RowItemLoader icon width="w-1.5/12" />
				<RowItemLoader icon width="w-1.5/12" />
				<RowItemLoader icon width="w-1.5/12" />
				<RowItemLoader icon width="w-1.5/12" />
			</div>
		</div>
	);
}
