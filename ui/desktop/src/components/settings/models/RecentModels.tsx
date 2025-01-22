import React, { useState, useEffect } from "react"
import { Clock } from 'lucide-react';
import { Model } from "./ModelContext"
import { useHandleModelSelection } from "./utils";
import { useModel } from "./ModelContext";


const MAX_RECENT_MODELS = 3

export function useRecentModels() {
    const [recentModels, setRecentModels] = useState<Model[]>([])

    useEffect(() => {
        const storedModels = localStorage.getItem("recentModels")
        if (storedModels) {
            setRecentModels(JSON.parse(storedModels))
        }
    }, [])

    const addRecentModel = (model: Model) => {
        const modelWithTimestamp = { ...model, lastUsed: new Date().toISOString() }; // Add lastUsed field
        setRecentModels((prevModels) => {
            const updatedModels = [modelWithTimestamp, ...prevModels.filter((m) => m.name !== model.name)].slice(0, MAX_RECENT_MODELS);

            localStorage.setItem("recentModels", JSON.stringify(updatedModels));
            return updatedModels;
        });
    };

    return { recentModels, addRecentModel }
}

export function RecentModels() {
    const { recentModels } = useRecentModels(); // Access the recent models from the hook

    return (
        <div className="space-y-2">
            {recentModels.map((model) => (
                <div
                    key={model.name}
                    className="flex items-center justify-between p-4 rounded-lg border border-muted-foreground/20 bg-background hover:bg-muted/50 transition-colors"
                >
                    <div className="space-y-1">
                        <p className="font-medium">{model.name}</p>
                        <p className="text-sm text-muted-foreground">{model.provider}</p>
                    </div>
                    <div className="flex items-center text-sm text-muted-foreground">
                        <Clock className="w-4 h-4 mr-2" />
                        {model.lastUsed ? new Date(model.lastUsed).toLocaleString() : "N/A"}
                    </div>
                </div>
            ))}
        </div>
    );
}

export function RecentModelsRadio() {
    const { recentModels } = useRecentModels(); // Access recent models
    const handleModelSelection = useHandleModelSelection(); // Access the model selection handler
    const { currentModel } = useModel(); // Get the current selected model
    const [selectedModel, setSelectedModel] = useState<string | null>(null); // Track the currently selected model

    // Initialize selectedModel with the currentModel on component mount
    useEffect(() => {
        if (currentModel) {
            setSelectedModel(currentModel.name);
        }
    }, [currentModel]);

    const handleRadioChange = async (model: Model) => {
        if (selectedModel === model.name) {
            // Display feedback for already selected model
            console.log(`Model "${model.name}" is already active.`);
            return;
        }

        setSelectedModel(model.name); // Update the selected model locally
        await handleModelSelection(model, "RecentModels"); // Switch the model using the handler
    };

    return (
        <div className="space-y-4">
            <h2 className="text-xl font-semibold">Recent Models</h2>
            {recentModels.map((model) => (
                <label
                    key={model.name}
                    className={`flex items-center justify-between p-4 rounded-lg bg-background hover:bg-muted/50 transition-colors cursor-pointer ${
                        selectedModel === model.name ? "bg-indigo-100" : ""
                    }`}
                >
                    <div className="space-y-1">
                        <p className="font-medium">{model.name}</p>
                        <p className="text-sm text-muted-foreground">{model.provider}</p>
                    </div>
                    <input
                        type="radio"
                        name="recentModels"
                        value={model.name}
                        checked={selectedModel === model.name}
                        onChange={() => handleRadioChange(model)} // Trigger model selection on change
                        className="form-radio h-4 w-4 text-indigo-600 transition duration-150 ease-in-out focus:ring-0 focus:outline-none"
                    />
                </label>
            ))}
        </div>
    );
}