import { TransformedTraceRow } from "@/types/trace";
import Evaluations from "../evaluations";

export default function ExtraTabs({
	tabKey,
	trace,
}: {
	tabKey: string;
	trace: TransformedTraceRow;
}) {
	if (tabKey === "Evaluation") {
		return <Evaluations trace={trace} />;
	}

	return null;
}
