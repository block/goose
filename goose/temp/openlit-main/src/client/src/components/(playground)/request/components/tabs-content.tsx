import { objectEntries } from "@/utils/object";
import { isArray, isPlainObject } from "lodash";
import ContentDataItem from "./content-data";

export default function TabsContent({
	dataKey,
	dataValue,
}: {
	dataKey: string;
	dataValue: Record<string, any> | string[] | Record<string, any>[];
}) {
	return isArray(dataValue)
		? dataValue.map((datumValue, index) => (
				<section key={`${dataKey}-${index}`}>
					{index !== 0 ? (
						<div className="py-1 px-2 dark:bg-stone-800"></div>
					) : null}
					{isPlainObject(datumValue) ? (
						objectEntries(datumValue).map(([key, value]) => (
							<ContentDataItem
								key={key.toString()}
								dataKey={key.toString()}
								dataValue={value}
							/>
						))
					) : (
						<ContentDataItem dataKey={datumValue} />
					)}
				</section>
		  ))
		: objectEntries(dataValue).map(([key, value]) => (
				<ContentDataItem key={key} dataKey={key} dataValue={value} />
		  ));
}
