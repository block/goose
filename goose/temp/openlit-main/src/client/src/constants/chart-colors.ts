import { shuffle } from "lodash";

const COLORS = [
	"violet-600",
	"orange-500",
	"yellow-300",
	"lime-300",
	"green-600",
	"emerald-950",
	"sky-500",
	"fuchsia-700",
	"rose-800",
	"cyan-200",
	"pink-600",
];

export const getChartColors = (length: number) => {
	return shuffle(COLORS).slice(0, length);
};
