"use client";

import { useState, useEffect } from "react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import { Input } from "@/components/ui/input";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Trash2Icon } from "lucide-react";
import getMessage from "@/constants/messages";
import { ModelMetadata } from "@/types/openground";
import useFetchWrapper from "@/utils/hooks/useFetchWrapper";
import { toast } from "sonner";

interface CustomModel extends ModelMetadata {
	id: string; // UUID from database
	model_id: string; // Model identifier like "gpt-4o"
}

interface ModelEditorPanelProps {
	model: ModelMetadata | null;
	provider: string | null;
	isCustomModel: boolean;
	isAddingNew: boolean;
	onSave: () => void;
	onDelete: () => void;
	onCancel: () => void;
}

export default function ModelEditorPanel({
	model,
	provider,
	isCustomModel,
	isAddingNew,
	onSave,
	onDelete,
	onCancel,
}: ModelEditorPanelProps) {
	const { fireRequest: fireSaveRequest, isLoading: saving } = useFetchWrapper();
	const { fireRequest: fireDeleteRequest, isLoading: deleting } = useFetchWrapper();

	const [showDeleteDialog, setShowDeleteDialog] = useState(false);
	const [formData, setFormData] = useState<Partial<CustomModel>>({
		id: "", // UUID (only for existing custom models)
		model_id: "", // Model identifier
		displayName: "",
		contextWindow: 4096,
		inputPricePerMToken: 0,
		outputPricePerMToken: 0,
		capabilities: [],
	});

	useEffect(() => {
		if (model && isCustomModel) {
			// Only allow editing custom models
			const customModel = model as CustomModel;
			setFormData({
				id: customModel.id || "", // UUID for existing custom models
				model_id: customModel.model_id,
				displayName: model.displayName,
				contextWindow: model.contextWindow,
				inputPricePerMToken: model.inputPricePerMToken,
				outputPricePerMToken: model.outputPricePerMToken,
				capabilities: model.capabilities || [],
			});
		} else if (isAddingNew) {
			// Reset form for new model
			setFormData({
				id: "",
				model_id: "",
				displayName: "",
				contextWindow: 4096,
				inputPricePerMToken: 0,
				outputPricePerMToken: 0,
				capabilities: [],
			});
		}
	}, [model, isAddingNew, isCustomModel]);


	const handleSave = () => {
		if (!formData.model_id || !formData.displayName || !provider) {
			toast.error(getMessage().OPENGROUND_MODEL_ID + " and " + getMessage().OPENGROUND_MODEL_DISPLAY_NAME + " are required");
			return;
		}

		const payload = {
			provider: provider,
			model: {
				model_id: formData.model_id,
				displayName: formData.displayName,
				contextWindow: formData.contextWindow || 4096,
				inputPricePerMToken: formData.inputPricePerMToken || 0,
				outputPricePerMToken: formData.outputPricePerMToken || 0,
				capabilities: formData.capabilities || [],
			},
			// Send UUID id for updates of existing custom models, undefined for new models
			id: formData.id || undefined,
		};

		fireSaveRequest({
			requestType: "POST",
			url: "/api/openground/models",
			body: JSON.stringify(payload),
			successCb: () => {
				toast.success(getMessage().OPENGROUND_MODEL_SAVED_SUCCESS, {
					id: "model-saved",
				});
				onSave();
			},
			failureCb: (err?: string) => {
				toast.error(err || getMessage().OPERATION_FAILED, {
					id: "model-save-error",
				});
			},
		});
	};

	const handleDelete = () => {
		const id = (model as CustomModel)?.id;
		if (!id) return;

		fireDeleteRequest({
			requestType: "DELETE",
			url: `/api/openground/models?id=${id}`,
			successCb: () => {
				toast.success(getMessage().OPENGROUND_MODEL_DELETED_SUCCESS, {
					id: "model-deleted",
				});
				setShowDeleteDialog(false);
				onDelete();
			},
			failureCb: (err?: string) => {
				toast.error(err || getMessage().OPERATION_FAILED, {
					id: "model-delete-error",
				});
			},
		});
	};

	return (
		<div className="p-6 max-w-3xl">
			<Card>
				<CardHeader>
					<div className="flex items-center justify-between">
						<CardTitle className="text-lg">
							{isAddingNew
								? getMessage().OPENGROUND_ADD_CUSTOM_MODEL
								: getMessage().OPENGROUND_EDIT_MODEL}
						</CardTitle>
					</div>
				</CardHeader>

				<CardContent className="space-y-4">
					{/* Model ID */}
					<div className="space-y-2">
						<Label htmlFor="model-id">{getMessage().OPENGROUND_MODEL_ID}*</Label>
						<Input
							id="model-id"
							placeholder="gpt-4o-custom"
							value={formData.model_id}
							onChange={(e) => setFormData({ ...formData, model_id: e.target.value })}
							disabled={!isAddingNew}
						/>
					</div>

					{/* Display Name */}
					<div className="space-y-2">
						<Label htmlFor="display-name">{getMessage().OPENGROUND_MODEL_DISPLAY_NAME}*</Label>
						<Input
							id="display-name"
							placeholder="GPT-4o Custom"
							value={formData.displayName}
							onChange={(e) => setFormData({ ...formData, displayName: e.target.value })}
						/>
					</div>

					{/* Context Window */}
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

					{/* Pricing */}
					<div className="grid grid-cols-2 gap-4">
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

					{/* Capabilities */}
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
						<p className="text-xs text-stone-500 dark:text-stone-400">
							Comma-separated list of model capabilities
						</p>
					</div>

					{/* Actions */}
					{(
						<div className="flex gap-2 pt-4">
							<Button onClick={handleSave} disabled={saving} className="flex-1">
								{saving ? getMessage().SAVING : getMessage().OPENGROUND_SAVE_MODEL}
							</Button>
							{isCustomModel && !isAddingNew && (
								<Button
									variant="destructive"
									onClick={() => setShowDeleteDialog(true)}
									disabled={deleting}
								>
									<Trash2Icon className="h-4 w-4 mr-2" />
									{getMessage().DELETE}
								</Button>
							)}
							<Button variant="outline" onClick={onCancel}>
								{getMessage().CANCEL}
							</Button>
						</div>
					)}
				</CardContent>
			</Card>

			{/* Delete Confirmation Dialog */}
			<Dialog open={showDeleteDialog} onOpenChange={setShowDeleteDialog}>
				<DialogContent>
					<DialogHeader>
						<DialogTitle>{getMessage().OPENGROUND_DELETE_MODEL}</DialogTitle>
						<DialogDescription>
							{getMessage().OPENGROUND_DELETE_MODEL_CONFIRMATION}
						</DialogDescription>
					</DialogHeader>
					<DialogFooter>
						<Button variant="outline" onClick={() => setShowDeleteDialog(false)}>
							{getMessage().CANCEL}
						</Button>
						<Button variant="destructive" onClick={handleDelete} disabled={deleting}>
							{deleting ? getMessage().LOADING : getMessage().DELETE}
						</Button>
					</DialogFooter>
				</DialogContent>
			</Dialog>
		</div>
	);
}
