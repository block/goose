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
                        <p className="text-sm font-medium">{model.name}</p>
                        <p className="text-xs text-muted-foreground">{model.provider}</p>
                    </div>
                    <div className="flex items-center text-xs text-muted-foreground">
                        <Clock className="w-3.5 h-3.5 mr-2" />
                        {model.lastUsed ? new Date(model.lastUsed).toLocaleString() : "N/A"}
                    </div>
                </div>
            ))}
        </div>
    );
}

export function RecentModelsRadio() {
    const { recentModels } = useRecentModels();
    const handleModelSelection = useHandleModelSelection();
    const { currentModel } = useModel();
    const [selectedModel, setSelectedModel] = useState<string | null>(null);

    useEffect(() => {
        if (currentModel) {
            setSelectedModel(currentModel.name);
        }
    }, [currentModel]);

    const handleRadioChange = async (model: Model) => {
        if (selectedModel === model.name) {
            console.log(`Model "${model.name}" is already active.`);
            return;
        }

        setSelectedModel(model.name);
        await handleModelSelection(model, "RecentModels");
    };

    return (
        <div className="space-y-2">
            <h3 className="text-base font-medium dark:text-white mb-2">Recent Models</h3>
            {recentModels.map((model) => (
                <label
                    key={model.name}
                    className={`bg-white dark:bg-gray-800 rounded-lg p-4 mb-2 flex items-center justify-between cursor-pointer hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors ${
                        selectedModel === model.name ? "bg-gray-50 dark:bg-gray-700" : ""
                    }`}
                >
                    <div className="flex-1">
                        <p className="text-base font-medium dark:text-white">{model.name}</p>
                        <p className="text-gray-500 dark:text-gray-400 text-xs mt-1">{model.provider}</p>
                    </div>
                    <input
                        type="radio"
                        name="recentModels"
                        value={model.name}
                        checked={selectedModel === model.name}
                        onChange={() => handleRadioChange(model)}
                        className="form-radio h-4 w-4 text-indigo-600 transition duration-150 ease-in-out focus:ring-0 focus:outline-none ml-4"
                    />
                </label>
            ))}
        </div>
    );
}