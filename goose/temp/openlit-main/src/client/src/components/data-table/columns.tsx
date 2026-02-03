import { ReactNode } from "react";

export type Columns<T extends string, TransformedT> = Partial<
	Record<
		T,
		{
			header: () => ReactNode;
			cell: ({ row, extraFunctions }: { row: TransformedT, extraFunctions: Record<string, any> }) => ReactNode;
			enableHiding?: boolean;
		}
	>
>;
