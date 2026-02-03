import { MouseEventHandler } from "react";
import {
	Pagination,
	PaginationContent,
	PaginationItem,
	PaginationNext,
	PaginationPrevious,
} from "@/components/ui/pagination";
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "@/components/ui/select";

type PaginationProps = {
	currentPage: number;
	currentSize: number;
	totalPage: number;
	onClickPageAction: (dir: -1 | 1) => void;
	onClickPageLimit: (size: number) => void;
};

const PageSizes = [10, 25, 50];

export default function TracesPagination(props: PaginationProps) {
	const onClickAction: MouseEventHandler = (ev) => {
		const { action } = (ev.currentTarget as HTMLButtonElement).dataset;
		if (action === "previous") {
			props.onClickPageAction(-1);
		} else {
			props.onClickPageAction(1);
		}
	};

	const onSizeChange = (size: string) => {
		props.onClickPageLimit(parseInt(size, 10));
	};

	const firstPage = props.currentPage === 1;
	const lastPage =
		props.totalPage === 0 || props.currentPage === props.totalPage;

	return (
		<div className="flex gap-4 self-start">
			<div className="flex align-end justify-center">
				<p className="text-xs shrink-0 mr-1 self-center text-stone-950 dark:text-stone-100">
					Size :{" "}
				</p>
				<div className="w-[100px]">
					<Select
						onValueChange={onSizeChange}
						defaultValue={`${props.currentSize}`}
					>
						<SelectTrigger
							id="model"
							className="items-center [&_[data-description]]:hidden text-stone-950 dark:text-stone-100 h-auto py-1"
						>
							<SelectValue
								placeholder={`${props.currentSize}`}
								defaultValue={`${props.currentSize}`}
							/>
						</SelectTrigger>
						<SelectContent>
							{PageSizes.map((size: number) => (
								<SelectItem
									key={size}
									value={`${size}`}
									className="outline-none"
								>
									<div className="flex items-start text-muted-foreground ">
										<div className="grid">
											<p>{size}</p>
										</div>
									</div>
								</SelectItem>
							))}
						</SelectContent>
					</Select>
				</div>
			</div>
			<Pagination className="w-auto shrink-0 m-0">
				<PaginationContent>
					<PaginationItem>
						<PaginationPrevious
							className={`py-1 h-auto ${
								firstPage
									? "pointer-events-none cursor-not-allowed text-stone-400"
									: "text-stone-950 dark:text-stone-100"
							}`}
							data-action={"previous"}
							onClick={onClickAction}
							aria-disabled={firstPage}
						/>
					</PaginationItem>
					<PaginationItem>
						<div className="flex items-center text-sm text-stone-950 dark:text-stone-100">
							{props.currentPage} of {props.totalPage || 1}
						</div>
					</PaginationItem>
					<PaginationItem>
						<PaginationNext
							className={`py-1 h-auto ${
								lastPage
									? "pointer-events-none cursor-not-allowed text-stone-400"
									: "text-stone-950 dark:text-stone-100"
							}`}
							data-action={"next"}
							onClick={onClickAction}
							aria-disabled={lastPage}
						/>
					</PaginationItem>
				</PaginationContent>
			</Pagination>
		</div>
	);
}
