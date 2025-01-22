import React, { useState, useEffect } from 'react';
import { Button } from "../../ui/button";
import { Input } from "../../ui/input";
import Select from 'react-select';
import { Plus } from 'lucide-react';
import {createSelectedModel, useHandleModelSelection} from "./utils";
import { useActiveKeys } from "../api_keys/ActiveKeysContext";
import { goose_models } from "./hardcoded_stuff";

export function AddModelInline() {
    const { activeKeys } = useActiveKeys(); // Access active keys from context

    // Convert active keys to dropdown options
    const providerOptions = activeKeys.map((key) => ({
        value: key.toLowerCase(),
        label: key,
    }));

    const [selectedProvider, setSelectedProvider] = useState<string | null>(null);
    const [modelName, setModelName] = useState<string>("");
    const [filteredModels, setFilteredModels] = useState([]);
    const [showSuggestions, setShowSuggestions] = useState(false);
    const handleModelSelection = useHandleModelSelection();

    // Filter models by selected provider and input text
    useEffect(() => {
        if (!selectedProvider || !modelName) {
            setFilteredModels([]);
            setShowSuggestions(false);
            return;
        }

        const filtered = goose_models
            .filter(
                (model) =>
                    model.provider.toLowerCase() === selectedProvider &&
                    model.name.toLowerCase().includes(modelName.toLowerCase())
            )
            .slice(0, 5); // Limit suggestions to top 5
        setFilteredModels(filtered);
        setShowSuggestions(filtered.length > 0);
    }, [modelName, selectedProvider]);

    const handleSubmit = () => {
        if (!selectedProvider || !modelName) {
            console.error("Both provider and model name are required.");
            return;
        }

        // Find the selected model from the filtered models
        const selectedModel = createSelectedModel(selectedProvider, modelName)

        // Trigger the model selection logic
        handleModelSelection(selectedModel, "AddModelInline");

        // Reset form state
        setSelectedProvider(null); // Clear the provider selection
        setModelName(""); // Clear the model name
        setFilteredModels([]);
        setShowSuggestions(false);
    };

    const handleSelectSuggestion = (suggestion) => {
        setModelName(suggestion.name);
        setShowSuggestions(false); // Hide suggestions after selection
    };

    const handleBlur = () => {
        setTimeout(() => setShowSuggestions(false), 150); // Delay to allow click to register
    };

    return (
        <div className="mb-6">
            <form className="grid grid-cols-[1.5fr_2fr_auto] gap-4 items-center">
                <Select
                    options={providerOptions}
                    value={providerOptions.find((option) => option.value === selectedProvider) || null}
                    onChange={(option) => {
                        setSelectedProvider(option?.value || null);
                        setModelName(""); // Clear model name when provider changes
                        setFilteredModels([]);
                    }}
                    placeholder="Select provider"
                    isClearable
                    styles={{
                        control: (base) => ({
                            ...base,
                            minWidth: "200px",
                            backgroundColor: "#1a1b1e",  // Dark background
                            borderColor: "#2a2b2e",
                            color: "#ffffff",
                        }),
                        menu: (base) => ({
                            ...base,
                            backgroundColor: "#1a1b1e",  // Dark solid background
                            boxShadow: "0 4px 6px -1px rgba(0, 0, 0, 0.1), 0 2px 4px -1px rgba(0, 0, 0, 0.06)",
                            border: "1px solid #2a2b2e",
                            // Force solid background
                            background: "#1a1b1e",
                        }),
                        menuList: (base) => ({
                            ...base,
                            backgroundColor: "#1a1b1e",  // Dark solid background
                            background: "#1a1b1e",  // Force solid background
                            padding: "4px",
                        }),
                        option: (base, state) => ({
                            ...base,
                            backgroundColor: state.isFocused 
                                ? "#2a2b2e"  // Slightly lighter when focused
                                : "#1a1b1e",  // Dark background
                            color: "#ffffff",  // White text
                            cursor: "pointer",
                            // Force solid background
                            background: state.isFocused 
                                ? "#2a2b2e"
                                : "#1a1b1e",
                            ":hover": {
                                backgroundColor: "#2a2b2e",  // Same as focused
                                color: "#ffffff",
                                background: "#2a2b2e",  // Force solid background
                            },
                            padding: "8px",
                            margin: "2px 0",
                            borderRadius: "4px",
                        }),
                        singleValue: (base) => ({
                            ...base,
                            color: "#ffffff",  // White text
                        }),
                        input: (base) => ({
                            ...base,
                            color: "#ffffff",  // White text
                        }),
                        placeholder: (base) => ({
                            ...base,
                            color: "#9ca3af",  // Gray text
                        }),
                        dropdownIndicator: (base) => ({
                            ...base,
                            color: "#9ca3af",  // Gray color for the dropdown arrow
                            ":hover": {
                                color: "#ffffff",  // White on hover
                            },
                        }),
                        indicatorSeparator: (base) => ({
                            ...base,
                            backgroundColor: "#2a2b2e",  // Dark separator
                        }),
                    }}
                    theme={(theme) => ({
                        ...theme,
                        colors: {
                            ...theme.colors,
                            primary: '#2a2b2e',
                            primary75: '#2a2b2e',
                            primary50: '#2a2b2e',
                            primary25: '#2a2b2e',
                            neutral0: '#1a1b1e',
                            neutral5: '#1a1b1e',
                            neutral10: '#2a2b2e',
                            neutral20: '#2a2b2e',
                            neutral30: '#3a3b3e',
                            neutral40: '#ffffff',
                            neutral50: '#ffffff',
                            neutral60: '#ffffff',
                            neutral70: '#ffffff',
                            neutral80: '#ffffff',
                            neutral90: '#ffffff',
                        },
                    })}
                />
                <div className="relative" style={{ minWidth: "150px", maxWidth: "250px" }}>
                    <Input
                        type="text"
                        placeholder="Model name"
                        value={modelName}
                        onChange={(e) => setModelName(e.target.value)}
                        onBlur={handleBlur}
                    />
                    {showSuggestions && (
                        <div className="absolute z-10 w-full mt-1 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-md shadow-lg">
                            {filteredModels.map((model) => (
                                <div
                                    key={model.id}
                                    className="p-2 cursor-pointer hover:bg-gray-100 dark:hover:bg-gray-700 dark:text-white"
                                    onClick={() => handleSelectSuggestion(model)}
                                >
                                    {model.name}
                                </div>
                            ))}
                        </div>
                    )}
                </div>
                <Button type="button" className="bg-black text-white hover:bg-black/90" onClick={handleSubmit}>
                    <Plus className="mr-2 h-4 w-4" /> Add Model
                </Button>
            </form>
        </div>
    );
}
