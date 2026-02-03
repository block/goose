import NumberStats from "./number-stats";
import dynamic from "next/dynamic";
const Operations = dynamic(() => import("./operations"), {
	ssr: false,
});

export default function VectorDashboard() {
	return (
		<>
			<NumberStats />
			<Operations />
		</>
	);
}
