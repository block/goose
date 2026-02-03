"use client";

import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import { Input } from "@/components/ui/input";
import { useState, useEffect } from "react";
import { toast } from "sonner";
import { PlusIcon, PencilIcon, Trash2Icon } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import useFetchWrapper from "@/utils/hooks/useFetchWrapper";
import getMessage from "@/constants/messages";
import { ModelMetadata } from "@/types/openground";
import {
	Table,
	TableBody,
	TableCell,
	TableHead,
	TableHeader,
	TableRow,
} from "@/components/ui/table";

interface ModelManagementDialogProps {
	providerId: string;
	open: boolean;
	onOpenChange: (open: boolean) => void;
	onModelAdded?: () => void;
}

interface CustomModel extends ModelMetadata {
	customId?: string;
}

export default function ModelManagementDialog({
	providerId,
	open,
	onOpenChange,
	onModelAdded,
}: ModelManagementDialogProps) {
	const { data: customModels, fireRequest: fireGetRequest, isLoading: loadingModels } =
		useFetchWrapper<CustomModel[]>();
	const { fireRequest: fireSaveRequest, isLoading: saving } = useFetchWrapper();
	const { fireRequest: fireDeleteRequest, isLoading: deleting } = useFetchWrapper();

	const [isEditing, setIsEditing] = useState(false);
	const [editingModel, setEditingModel] = useState<CustomModel | null>(null);
	const [formData, setFormData] = useState<Partial<ModelMetadata>>({
		id: "",
		displayName: "",
		contextWindow: 4096,
		inputPricePerMToken: 0,
		outputPricePerMToken: 0,
		capabilities: [],
	});

	useEffect(() => {
		if (open) {
			loadCustomModels();
		}
	}, [open, providerId]);

	const loadCustomModels = () => {
		fireGetRequest({
			requestType: "GET",
			url: `/api/openground/models?provider=${providerId}`,
			failureCb: (err?: string) => {
				console.error("Failed to load custom models:", err);
			},
		});
	};

	const handleSave = () => {
		if (!formData.id || !formData.displayName) {
			toast.error(getMessage().OPENGROUND_MODEL_ID + " and " + getMessage().OPENGROUND_MODEL_DISPLAY_NAME + " are required");
			return;
		}

		const payload = {
			provider: providerId,
			model: {
				id: formData.id,
				displayName: formData.displayName,
				contextWindow: formData.contextWindow || 4096,
				inputPricePerMToken: formData.inputPricePerMToken || 0,
				outputPricePerMToken: formData.outputPricePerMToken || 0,
				capabilities: formData.capabilities || [],
			},
			customId: editingModel?.customId,
		};

		fireSaveRequest({
			requestType: "POST",
			url: "/api/openground/models",
			body: JSON.stringify(payload),
			successCb: () => {
				toast.success(getMessage().OPENGROUND_MODEL_SAVED_SUCCESS, {
					id: "model-saved",
				});
				resetForm();
				loadCustomModels();
				onModelAdded?.();
			},
			failureCb: (err?: string) => {
				toast.error(err || getMessage().OPERATION_FAILED, {
					id: "model-save-error",
				});
			},
		});
	};

	const handleDelete = (model: CustomModel) => {
		if (!model.customId) return;

		fireDeleteRequest({
			requestType: "DELETE",
			url: `/api/openground/models?id=${model.customId}`,
			successCb: () => {
				toast.success(getMessage().OPENGROUND_MODEL_DELETED_SUCCESS, {
					id: "model-deleted",
				});
				loadCustomModels();
				onModelAdded?.();
			},
			failureCb: (err?: string) => {
				toast.error(err || getMessage().OPERATION_FAILED, {
					id: "model-delete-error",
				});
			},
		});
	};

	const handleEdit = (model: CustomModel) => {
		setEditingModel(model);
		setFormData({
			id: model.id,
			displayName: model.displayName,
			contextWindow: model.contextWindow,
			inputPricePerMToken: model.inputPricePerMToken,
			outputPricePerMToken: model.outputPricePerMToken,
			capabilities: model.capabilities || [],
		});
		setIsEditing(true);
	};

	const resetForm = () => {
		setFormData({
			id: "",
			displayName: "",
			contextWindow: 4096,
			inputPricePerMToken: 0,
			outputPricePerMToken: 0,
			capabilities: [],
		});
		setIsEditing(false);
		setEditingModel(null);
	};

	return (
		<Dialog open={open} onOpenChange={onOpenChange}>
			<DialogContent className="sm:max-w-[700px] max-h-[80vh] overflow-y-auto">
				<DialogHeader>
					<DialogTitle className="flex items-center gap-2">
						{getMessage().OPENGROUND_MANAGE_MODELS}
					</DialogTitle>
					<DialogDescription>
						{isEditing ? getMessage().OPENGROUND_EDIT_MODEL : getMessage().OPENGROUND_ADD_NEW_MODEL}
					</DialogDescription>
				</DialogHeader>

				<div className="space-y-6 py-4">
					{/* Form Section */}
					<div className="space-y-4 p-4 border rounded-lg bg-stone-50 dark:bg-stone-900">
						<div className="grid grid-cols-2 gap-4">
							<div className="space-y-2">
								<Label htmlFor="model-id">{getMessage().OPENGROUND_MODEL_ID}*</Label>
								<Input
									id="model-id"
									placeholder="gpt-4o-custom"
									value={formData.id}
									onChange={(e) => setFormData({ ...formData, id: e.target.value })}
									disabled={isEditing}
								/>
							</div>
							<div className="space-y-2">
								<Label htmlFor="model-display-name">{getMessage().OPENGROUND_MODEL_DISPLAY_NAME}*</Label>
								<Input
									id="model-display-name"
									placeholder="GPT-4o Custom"
									value={formData.displayName}
									onChange={(e) => setFormData({ ...formData, displayName: e.target.value })}
								/>
							</div>
						</div>

						<div className="grid grid-cols-3 gap-4">
							<div className="space-y-2">
								<Label htmlFor="context-window">{getMessage().OPENGROUND_CONTEXT_WINDOW}</Label>
								<Input
									id="context-window"
									type="number"
									placeholder="4096"
									value={formData.contextWindow}
									onChange={(e) => setFormData({ ...formData, contextWindow: parseInt(e.target.value) || 0 })}
								/>
							</div>
							<div className="space-y-2">
								<Label htmlFor="input-price">{getMessage().OPENGROUND_INPUT_PRICE_PER_M_TOKENS}</Label>
								<Input
									id="input-price"
									type="number"
									step="0.001"
									placeholder="0.5"
									value={formData.inputPricePerMToken}
									onChange={(e) => setFormData({ ...formData, inputPricePerMToken: parseFloat(e.target.value) || 0 })}
								/>
							</div>
							<div className="space-y-2">
								<Label htmlFor="output-price">{getMessage().OPENGROUND_OUTPUT_PRICE_PER_M_TOKENS}</Label>
								<Input
									id="output-price"
									type="number"
									step="0.001"
									placeholder="1.5"
									value={formData.outputPricePerMToken}
									onChange={(e) => setFormData({ ...formData, outputPricePerMToken: parseFloat(e.target.value) || 0 })}
								/>
							</div>
						</div>

						<div className="space-y-2">
							<Label htmlFor="capabilities">{getMessage().OPENGROUND_MODEL_CAPABILITIES}</Label>
							<Input
								id="capabilities"
								placeholder="function-calling, vision, streaming"
								value={Array.isArray(formData.capabilities) ? formData.capabilities.join(", ") : ""}
								onChange={(e) => {
									const caps = e.target.value.split(",").map(s => s.trim()).filter(Boolean);
									setFormData({ ...formData, capabilities: caps });
								}}
							/>
						</div>

						<div className="flex gap-2">
							<Button onClick={handleSave} disabled={saving} className="flex-1">
								{saving ? getMessage().SAVING : getMessage().OPENGROUND_SAVE_MODEL}
							</Button>
							{isEditing && (
								<Button variant="outline" onClick={resetForm} disabled={saving}>
									{getMessage().CANCEL}
								</Button>
							)}
						</div>
					</div>

					{/* Existing Custom Models List */}
					<div className="space-y-2">
						<h4 className="font-medium text-sm">{getMessage().OPENGROUND_CUSTOM_MODEL}s</h4>
						{loadingModels ? (
							<div className="text-center py-4 text-sm text-stone-500">
								{getMessage().LOADING}...
							</div>
						) : customModels && customModels.length > 0 ? (
							<Table>
								<TableHeader>
									<TableRow>
										<TableHead>{getMessage().OPENGROUND_MODEL_ID}</TableHead>
										<TableHead>{getMessage().OPENGROUND_MODEL_DISPLAY_NAME}</TableHead>
										<TableHead>{getMessage().OPENGROUND_CONTEXT_WINDOW}</TableHead>
										<TableHead>{getMessage().OPENGROUND_INPUT_PRICE_PER_M_TOKENS}</TableHead>
										<TableHead>{getMessage().OPENGROUND_OUTPUT_PRICE_PER_M_TOKENS}</TableHead>
										<TableHead className="text-right">{getMessage().ACTIONS}</TableHead>
									</TableRow>
								</TableHeader>
								<TableBody>
									{customModels.map((model) => (
										<TableRow key={model.customId || model.id}>
											<TableCell className="font-mono text-xs">{model.id}</TableCell>
											<TableCell>{model.displayName}</TableCell>
											<TableCell>{model.contextWindow.toLocaleString()}</TableCell>
											<TableCell>${model.inputPricePerMToken}</TableCell>
											<TableCell>${model.outputPricePerMToken}</TableCell>
											<TableCell className="text-right">
												<div className="flex gap-1 justify-end">
													<Button
														variant="ghost"
														size="icon"
														className="h-8 w-8"
														onClick={() => handleEdit(model)}
													>
														<PencilIcon className="h-4 w-4" />
													</Button>
													<Button
														variant="ghost"
														size="icon"
														className="h-8 w-8 text-red-600 hover:text-red-700"
														onClick={() => handleDelete(model)}
														disabled={deleting}
													>
														<Trash2Icon className="h-4 w-4" />
													</Button>
												</div>
											</TableCell>
										</TableRow>
									))}
								</TableBody>
							</Table>
						) : (
							<div className="text-center py-8 text-sm text-stone-500 border rounded-lg">
								{getMessage().OPENGROUND_NO_CUSTOM_MODELS_YET}
							</div>
						)}
					</div>
				</div>

				<DialogFooter>
					<Button variant="outline" onClick={() => onOpenChange(false)}>
						{getMessage().CLOSE}
					</Button>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	);
}
