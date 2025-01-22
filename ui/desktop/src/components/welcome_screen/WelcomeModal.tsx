import React, { useEffect, useState } from "react";
import { ProviderSetupModal } from "../settings/ProviderSetupModal";
import { Card } from "../ui/card";
import { ProviderList } from "../settings/providers/old_stuff/ProvidersList";
import { getProvidersList, Provider } from "../../utils/providerUtils";

export const WelcomeModal = ({
                                 selectedProvider,
                                 setSelectedProvider,
                                 onSubmit,
                             }: {
    selectedProvider: Provider | string | null;
    setSelectedProvider: React.Dispatch<React.SetStateAction<Provider | null>>;
    onSubmit: (apiKey: string) => void;
}) => {
    const [providers, setProviders] = useState<Provider[]>([]);
    const [error, setError] = useState<string | null>(null);

    useEffect(() => {
        const fetchProviders = async () => {
            try {
                const providerList = await getProvidersList();
                // Filter for only "anthropic" and "openai"
                const filteredProviders = providerList.filter((provider) =>
                    ["anthropic", "openai"].includes(provider.id)
                );
                setProviders(filteredProviders);
            } catch (err) {
                console.error("Failed to fetch providers:", err);
                setError("Unable to load providers. Please try again.");
            }
        };

        fetchProviders();
    }, []);

    return (
        <div className="fixed inset-0 bg-black/20 backdrop-blur-sm z-[9999]">
            {selectedProvider ? (
                <ProviderSetupModal
                    provider={selectedProvider.name}
                    onSubmit={onSubmit}
                    onCancel={() => setSelectedProvider(null)}
                    model={""}  // placeholder
                    endpoint={""}  // placeholder
                />
            ) : (
                <Card className="fixed top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[440px] bg-white dark:bg-gray-800 rounded-xl shadow-xl overflow-hidden p-[16px] pt-[24px]">
                    <h2 className="text-2xl font-medium mb-6 dark:text-white">
                        Select a Provider
                    </h2>
                    {error ? (
                        <p className="text-center text-red-500">{error}</p>
                    ) : (
                        <ProviderList
                            providers={providers} // Use state here
                            onProviderSelect={setSelectedProvider}
                        />
                    )}
                </Card>
            )}
        </div>
    );
};

