"use client";

import React, { useEffect } from "react";
import {
	BarChartWidget,
	PieChartWidget,
	StatCardWidget,
	WidgetType,
	MarkdownWidget,
	ChartWidget,
	LineChartWidget,
	AreaChartWidget,
	ColorTheme,
} from "../types";
import { useEditWidget } from "../hooks/useEditWidget";
import { Trash2, Info } from "lucide-react";
import CodeEditor from "./code-editor";

import {
	Sheet,
	SheetContent,
	SheetHeader,
	SheetTitle,
	SheetDescription,
	SheetFooter,
} from "@/components/ui/sheet";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "@/components/ui/select";
import { Textarea } from "@/components/ui/textarea";
import { useDashboard } from "../context/dashboard-context";
import QueryDebugger from "./query-debugger";
import { ColorSelector } from "./color-selector";
import MarkdownWidgetComponent from "../widgets/markdown-widget";
import { Tooltip, TooltipTrigger, TooltipPortal, TooltipContent } from "@/components/ui/tooltip";
import { DEFAULT_PRIMARY_COLOR, SUPPORTED_WIDGETS } from "../constants";

interface NonMarkdownConfig {
	query: string;
}

interface MarkdownConfig {
	content: string;
	showPreview?: boolean;
	colorTheme?: ColorTheme;
}

interface EditWidgetSheetProps {
	editorLanguage?: string;
}

export const EditWidgetSheet: React.FC<EditWidgetSheetProps> = ({
	editorLanguage = "clickhouse-sql",
}) => {
	const {
		currentTab,
		setCurrentTab,
		isFullscreenEditor,
		currentWidget,
		closeEditSheet,
		updateWidget,
		updateWidgetProperties,
		runQuery,
	} = useEditWidget();

	const { handleWidgetCrud, loadWidgetData } = useDashboard();
	const [queryResult, setQueryResult] = React.useState<any>(null);
	const [queryError, setQueryError] = React.useState<string | null>(null);
	const [isQueryLoading, setIsQueryLoading] = React.useState(false);

	// if (!currentWidget) return null;

	const handleEditorChange = (value: string | undefined) => {
		if (!currentWidget) return;

		if (currentWidget.type === WidgetType.MARKDOWN) {
			updateWidget(currentWidget.id, {
				config: {
					...(currentWidget as MarkdownWidget).config,
					content: value || '',
				},
			});
		} else {
			const config: NonMarkdownConfig = {
				...currentWidget.config,
				query: value || '',
			};
			updateWidget(currentWidget.id, { config });
		}
	};

	const handleMarkdownChange = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
		if (!currentWidget) return;

		updateWidget(currentWidget.id, {
			config: {
				...(currentWidget as MarkdownWidget).config,
				content: e.target.value,
			},
		});
	};

	const handleRunQuery = async () => {
		if (currentWidget?.type !== WidgetType.MARKDOWN && currentWidget?.config && 'query' in currentWidget.config) {
			setIsQueryLoading(true);
			setQueryError(null);
			try {
				const result = await runQuery(currentWidget.id, {
					userQuery: (currentWidget.config as NonMarkdownConfig).query,
				});
				setQueryResult(result.data);
				setQueryError(result.err);
			} catch (error) {
				setQueryError(
					error instanceof Error
						? error.message
						: "An error occurred while running the query"
				);
			} finally {
				setIsQueryLoading(false);
			}
		}
	};

	const handleSave = async () => {
		if (currentWidget && handleWidgetCrud) {
			try {
				await handleWidgetCrud(currentWidget);
				loadWidgetData(currentWidget.id);
				closeEditSheet();
			} catch (error) {
				console.error("Failed to save widget:", error);
			}
		} else {
			closeEditSheet();
		}
	};

	useEffect(() => {
		if (currentWidget?.id) {
			setCurrentTab("general");
			setQueryResult(null);
			setQueryError(null);
			setIsQueryLoading(false);
		}
	}, [currentWidget?.id]);

	return (
		<Sheet
			open={!!currentWidget}
			onOpenChange={(open) => {
				if (!open) {
					closeEditSheet();
				}
			}}
		>
			<SheetContent
				className={`${isFullscreenEditor
					? "w-full max-w-full h-full p-0"
					: "sm:max-w-md md:max-w-lg flex flex-col h-full"
					} bg-white dark:bg-stone-950 border-stone-200 dark:border-stone-800`}
			>
				<div className="flex flex-col h-full">
					<SheetHeader>
						<SheetTitle className="text-stone-900 dark:text-white flex items-center gap-2">
							<p className="text-lg font-semibold">{currentWidget?.title}</p>
							{currentWidget?.description && (
								<Tooltip delayDuration={0}>
									<TooltipTrigger asChild>
										<Info className="h-3 w-3" />
									</TooltipTrigger>
									<TooltipPortal>

										<TooltipContent>{currentWidget.description}</TooltipContent>
									</TooltipPortal>
								</Tooltip>
							)}
						</SheetTitle>
						<SheetDescription className="text-stone-600 dark:text-stone-300">
							Configure your widget
						</SheetDescription>
					</SheetHeader>

					{currentWidget && (
						<div className="flex-1 py-4 relative overflow-hidden">
							<Tabs value={currentTab} onValueChange={setCurrentTab} className="flex flex-col h-full w-full">
								<TabsList className="grid w-full grid-cols-3 bg-stone-200 dark:bg-stone-900 sticky top-0 z-10">
									<TabsTrigger
										value="general"
										className="data-[state=active]:bg-primary data-[state=active]:text-stone-50 text-stone-900 dark:text-white"
									>
										General
									</TabsTrigger>
									<TabsTrigger
										value="query"
										className="data-[state=active]:bg-primary data-[state=active]:text-stone-50 text-stone-900 dark:text-white"
									>
										{currentWidget.type === WidgetType.MARKDOWN ? "Markdown" : "Query"}
									</TabsTrigger>
									<TabsTrigger
										value="appearance"
										className="data-[state=active]:bg-primary data-[state=active]:text-stone-50 text-stone-900 dark:text-white"
									>
										Appearance
									</TabsTrigger>
								</TabsList>

								{/* General Tab */}
								<TabsContent value="general" className="space-y-4 mt-4 overflow-y-auto flex flex-col overflow-y-auto">
									<div className="space-y-2">
										<Label htmlFor="title" className="text-stone-900 dark:text-white">Widget Title</Label>
										<Input
											id="title"
											value={currentWidget.title}
											onChange={(e) =>
												updateWidget(currentWidget.id, {
													title: e.target.value,
												})
											}
											className="bg-white dark:bg-stone-900 border-stone-200 dark:border-stone-700 dark:text-white"
										/>
									</div>

									<div className="space-y-2">
										<Label htmlFor="type" className="text-stone-900 dark:text-white">Widget Type</Label>
										<Select
											value={currentWidget.type}
											onValueChange={(value) =>
												updateWidget(currentWidget.id, {
													type: value as WidgetType,
												})
											}
										>
											<SelectTrigger className="bg-white dark:bg-stone-900 border-stone-200 dark:border-stone-700 dark:text-white">
												<SelectValue placeholder="Select widget type" />
											</SelectTrigger>
											<SelectContent className="bg-white dark:bg-stone-900 border-stone-200 dark:border-stone-700">
												{
													Object.entries(SUPPORTED_WIDGETS).map(([type, widget]) => (
														<SelectItem key={type} value={type} className="dark:text-white">
															<div className="flex items-center gap-2">
																<widget.icon className="h-4 w-4" />
																<span>{widget.name}</span>
															</div>
														</SelectItem>
													))
												}
											</SelectContent>
										</Select>
									</div>

									<div className="space-y-2">
										<Label htmlFor="description" className="text-stone-900 dark:text-white">Description</Label>
										<Textarea
											id="description"
											value={currentWidget.description}
											onChange={(e) =>
												updateWidget(currentWidget.id, {
													description: e.target.value,
												})
											}
											placeholder="Describe what this widget shows"
											className="bg-white dark:bg-stone-900 border-stone-200 dark:border-stone-700 dark:text-white"
										/>
									</div>

									{/* Stat Card specific fields */}
									{currentWidget.type === WidgetType.STAT_CARD && (
										<div className="space-y-4">
											<div className="grid grid-cols-3 gap-4">
												<div className="space-y-2">
													<Label htmlFor="prefix" className="text-stone-900 dark:text-white">Prefix</Label>
													<Input
														id="prefix"
														value={(currentWidget as StatCardWidget).properties.prefix}
														onChange={(e) =>
															updateWidgetProperties(currentWidget.id, {
																prefix: e.target.value,
															})
														}
														placeholder="$"
														className="bg-white dark:bg-stone-900 border-stone-200 dark:border-stone-700 dark:text-white"
													/>
												</div>
												<div className="space-y-2">
													<Label htmlFor="value" className="text-stone-900 dark:text-white">Value Path</Label>
													<Input
														id="value"
														value={(currentWidget as StatCardWidget).properties.value}
														onChange={(e) =>
															updateWidget(currentWidget.id, {
																properties: {
																	...currentWidget.properties,
																	value: e.target.value,
																},
															})
														}
														placeholder="1,234"
														className="bg-white dark:bg-stone-900 border-stone-200 dark:border-stone-700 dark:text-white"
													/>
												</div>
												<div className="space-y-2">
													<Label htmlFor="suffix" className="text-stone-900 dark:text-white">Suffix</Label>
													<Input
														id="suffix"
														value={(currentWidget as StatCardWidget).properties.suffix}
														onChange={(e) =>
															updateWidgetProperties(currentWidget.id, {
																suffix: e.target.value,
															})
														}
														placeholder="%"
														className="bg-white dark:bg-stone-900 border-stone-200 dark:border-stone-700 dark:text-white"
													/>
												</div>
											</div>

											<div className="grid grid-cols-3 gap-4">
												<div className="space-y-2">
													<Label htmlFor="trendPrefix" className="text-stone-900 dark:text-white">Trend Prefix</Label>
													<Input
														id="trendPrefix"
														value={(currentWidget as StatCardWidget).properties.trendPrefix}
														onChange={(e) =>
															updateWidgetProperties(currentWidget.id, {
																trendPrefix: e.target.value,
															})
														}
														placeholder="$"
														className="bg-white dark:bg-stone-900 border-stone-200 dark:border-stone-700 dark:text-white"
													/>
												</div>
												<div className="space-y-2">
													<Label htmlFor="trend" className="text-stone-900 dark:text-white">Trend Path</Label>
													<Input
														id="trend"
														value={(currentWidget as StatCardWidget).properties.trend}
														onChange={(e) =>
															updateWidgetProperties(currentWidget.id, {
																trend: e.target.value,
															})
														}
														placeholder="12%"
														className="bg-white dark:bg-stone-900 border-stone-200 dark:border-stone-700 dark:text-white"
													/>
												</div>
												<div className="space-y-2">
													<Label htmlFor="trendSuffix" className="text-stone-900 dark:text-white">Trend Suffix</Label>
													<Input
														id="trendSuffix"
														value={(currentWidget as StatCardWidget).properties.trendSuffix}
														onChange={(e) =>
															updateWidgetProperties(currentWidget.id, {
																trendSuffix: e.target.value,
															})
														}
														placeholder="%"
														className="bg-white dark:bg-stone-900 border-stone-200 dark:border-stone-700 dark:text-white"
													/>
												</div>
											</div>
										</div>
									)}

									{/* Chart specific fields */}
									{(currentWidget.type === WidgetType.BAR_CHART ||
										currentWidget.type === WidgetType.LINE_CHART ||
										currentWidget.type === WidgetType.AREA_CHART) && (
											<div className="space-y-4">
												<div className="space-y-2">
													<Label htmlFor="xAxis" className="text-stone-900 dark:text-white">X Axis</Label>
													<Input
														id="xAxis"
														value={
															(currentWidget as BarChartWidget | LineChartWidget | AreaChartWidget).properties.xAxis
														}
														onChange={(e) =>
															updateWidgetProperties(currentWidget.id, {
																xAxis: e.target.value,
															})
														}
														placeholder="date, category, etc."
														className="bg-white dark:bg-stone-900 border-stone-200 dark:border-stone-700 dark:text-white"
													/>
												</div>
												{currentWidget.type !== WidgetType.AREA_CHART && (
													<div className="space-y-2">
														<Label htmlFor="yAxis" className="text-stone-900 dark:text-white">Y Axis</Label>
														<Input
															id="yAxis"
															value={
																(currentWidget as BarChartWidget | LineChartWidget).properties.yAxis
															}
															onChange={(e) =>
																updateWidgetProperties(currentWidget.id, {
																	yAxis: e.target.value,
																})
															}
															placeholder="value, count, etc."
															className="bg-white dark:bg-stone-900 border-stone-200 dark:border-stone-700 dark:text-white"
														/>
													</div>
												)}
												{currentWidget.type === WidgetType.AREA_CHART && (
													<div className="space-y-2">
														<div className="flex items-center justify-between">
															<Label className="text-stone-900 dark:text-white">Y Axes</Label>
															<Button
																variant="outline"
																size="sm"
																onClick={() =>
																	updateWidgetProperties(currentWidget.id, {
																		yAxes: [
																			...(currentWidget as AreaChartWidget).properties.yAxes || [],
																			{
																				key: "",
																				color: DEFAULT_PRIMARY_COLOR,
																			},
																		],
																	})
																}
																className="border-stone-200 dark:border-stone-700 hover:bg-stone-100 dark:hover:bg-stone-800"
															>
																Add Y Axis
															</Button>
														</div>
														<div className="space-y-4">
															{(currentWidget as AreaChartWidget).properties.yAxes?.map(
																(yAxis, index) => (
																	<div key={index} className="flex items-end gap-2">
																		<div className="flex-1">
																			<Label htmlFor={`yAxis-${index}`} className="text-stone-900 dark:text-white">
																				Y Axis {index + 1}
																			</Label>
																			<Input
																				id={`yAxis-${index}`}
																				value={yAxis.key}
																				onChange={(e) => {
																					const newYAxes = [
																						...(currentWidget as AreaChartWidget).properties.yAxes,
																					];
																					newYAxes[index] = {
																						...newYAxes[index],
																						key: e.target.value,
																					};
																					updateWidgetProperties(currentWidget.id, {
																						yAxes: newYAxes,
																					});
																				}}
																				placeholder="value, count, etc."
																				className="bg-white dark:bg-stone-900 border-stone-200 dark:border-stone-700 dark:text-white"
																			/>
																		</div>
																		<div className="w-32">
																			<Label htmlFor={`yAxis-${index}-color`} className="text-stone-900 dark:text-white">
																				Color
																			</Label>
																			<ColorSelector
																				value={yAxis.color}
																				onChange={(value) => {
																					const newYAxes = [
																						...(currentWidget as AreaChartWidget).properties.yAxes,
																					];
																					newYAxes[index] = {
																						...newYAxes[index],
																						color: value,
																					};
																					updateWidgetProperties(currentWidget.id, {
																						yAxes: newYAxes,
																					});
																				}}
																			/>
																		</div>
																		<Button
																			variant="ghost"
																			size="icon"
																			className="h-10 w-10 hover:bg-stone-100 dark:hover:bg-stone-800 text-stone-600 dark:text-stone-400"
																			onClick={() => {
																				const newYAxes = [
																					...(currentWidget as AreaChartWidget).properties.yAxes,
																				];
																				newYAxes.splice(index, 1);
																				updateWidgetProperties(currentWidget.id, {
																					yAxes: newYAxes,
																				});
																			}}
																		>
																			<Trash2 className="h-4 w-4" />
																		</Button>
																	</div>
																)
															)}
														</div>
													</div>
												)}
											</div>
										)}

									{currentWidget.type === WidgetType.PIE_CHART && (
										<div className="space-y-4">
											<div className="grid grid-cols-3 gap-4">
												<div className="space-y-2">
													<Label htmlFor="labelPath" className="text-stone-900 dark:text-white">Label Path</Label>
													<Input
														id="labelPath"
														value={
															(currentWidget as PieChartWidget).properties
																.labelPath
														}
														onChange={(e) =>
															updateWidgetProperties(currentWidget.id, {
																labelPath: e.target.value,
															})
														}
														placeholder="$"
														className="bg-white dark:bg-stone-900 border-stone-200 dark:border-stone-700 dark:text-white"
													/>
												</div>
												<div className="space-y-2">
													<Label htmlFor="value" className="text-stone-900 dark:text-white">Value Path</Label>
													<Input
														id="value"
														value={
															(currentWidget as PieChartWidget).properties
																.valuePath
														}
														onChange={(e) =>
															updateWidget(
																(currentWidget as PieChartWidget).id,
																{
																	properties: {
																		...(currentWidget as PieChartWidget)
																			.properties,
																		valuePath: e.target.value,
																	},
																}
															)
														}
														placeholder="1,234"
														className="bg-white dark:bg-stone-900 border-stone-200 dark:border-stone-700 dark:text-white"
													/>
												</div>
											</div>
										</div>
									)}
								</TabsContent>

								{/* Config Tab */}
								<TabsContent value="query" className="space-y-4 mt-4 overflow-y-auto flex-col overflow-y-auto">
									{currentWidget.type !== WidgetType.MARKDOWN ? (
										<>
											<div className="space-y-2">
												<div className="flex justify-between items-center">
													<Label htmlFor="query" className="text-stone-900 dark:text-white">Query</Label>
												</div>
												<div className="border rounded-md h-[calc(100vh-400px)] bg-white dark:bg-stone-900 border-stone-200 dark:border-stone-700">
													<CodeEditor
														value={currentWidget.config && 'query' in currentWidget.config ? (currentWidget.config as NonMarkdownConfig).query : ""}
														onChange={handleEditorChange}
														language={editorLanguage}
													/>
												</div>
											</div>

											<div className="flex justify-end">
												<Button
													size="sm"
													onClick={handleRunQuery}
													className="bg-primary hover:bg-primary/90 text-white"
												>
													Run Query
												</Button>
											</div>
										</>
									) : (
										<div className="space-y-4">
											<div className="space-y-2">
												<Label htmlFor="content" className="text-stone-900 dark:text-white">Markdown Content</Label>
												<Tabs defaultValue="write" className="w-full">
													<TabsList className="grid w-full grid-cols-2 bg-stone-100 dark:bg-stone-900">
														<TabsTrigger
															value="write"
															className="data-[state=active]:bg-primary data-[state=active]:text-primary-foreground dark:text-white"
														>
															Write
														</TabsTrigger>
														<TabsTrigger
															value="preview"
															className="data-[state=active]:bg-primary data-[state=active]:text-primary-foreground dark:text-white"
														>
															Preview
														</TabsTrigger>
													</TabsList>
													<TabsContent value="write" className="mt-2">
														<Textarea
															id="content"
															value={currentWidget.config && 'content' in currentWidget.config ? (currentWidget.config as MarkdownConfig).content : ""}
															onChange={handleMarkdownChange}
															placeholder="Write your markdown content here..."
															className="min-h-[400px] font-mono bg-white dark:bg-stone-900 border-stone-200 dark:border-stone-700 dark:text-white dark:placeholder:text-stone-400"
														/>
													</TabsContent>
													<TabsContent value="preview" className="mt-2">
														<div className="min-h-[400px] p-4 bg-white dark:bg-stone-900 border border-stone-200 dark:border-stone-700 rounded-md overflow-y-auto">
															<MarkdownWidgetComponent widget={currentWidget as MarkdownWidget} />
														</div>
													</TabsContent>
												</Tabs>
											</div>
										</div>
									)}
								</TabsContent>

								{/* Appearance Tab */}
								<TabsContent value="appearance" className="space-y-4 mt-4 overflow-y-auto flex-col overflow-y-auto">
									{(currentWidget.type === WidgetType.STAT_CARD ||
										currentWidget.type === WidgetType.BAR_CHART ||
										currentWidget.type === WidgetType.LINE_CHART ||
										currentWidget.type === WidgetType.PIE_CHART ||
										currentWidget.type === WidgetType.TABLE ||
										currentWidget.type === WidgetType.MARKDOWN) && (
											<div className="space-y-2">
												<Label htmlFor="color" className="text-stone-900 dark:text-white">Color Theme</Label>
												<ColorSelector
													value={currentWidget.properties.color}
													onChange={(value) =>
														updateWidgetProperties(currentWidget.id, {
															color: value,
														})
													}
												/>
											</div>
										)}
									{
										currentWidget.type === WidgetType.AREA_CHART && (
											<div className="space-y-2 text-stone-500 dark:text-stone-400">
												No appearance options for area chart
											</div>
										)
									}
								</TabsContent>
							</Tabs>
						</div>
					)}

					<SheetFooter className="flex-shrink-0 border-t border-stone-200 dark:border-stone-800 bg-white/95 dark:bg-stone-950/95 backdrop-blur supports-[backdrop-filter]:bg-white/60 dark:supports-[backdrop-filter]:bg-stone-950/60 pt-4">
						<div className="flex justify-between w-full">
							<Button
								variant="outline"
								onClick={closeEditSheet}
								className="px-8 border-stone-200 dark:border-stone-700 hover:bg-stone-100 dark:hover:bg-stone-800 dark:text-white"
							>
								Cancel
							</Button>
							<Button
								onClick={handleSave}
								className="px-8 bg-primary dark:bg-primary hover:bg-primary/90 dark:hover:bg-primary/90 text-white dark:text-white"
							>
								Save Changes
							</Button>
						</div>
					</SheetFooter>
				</div>
				{
					currentTab === "query" && currentWidget?.type !== WidgetType.MARKDOWN && (
						<QueryDebugger
							data={queryResult}
							error={queryError || undefined}
							isLoading={isQueryLoading}
						/>
					)
				}
			</SheetContent>
		</Sheet>
	);
};

export default EditWidgetSheet;
