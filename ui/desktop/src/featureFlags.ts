interface FeatureFlags {
    whatCanGooseDoText: string;
    expandedToolsByDefault: boolean;
}

class FeatureFlagsManager {
    private static instance: FeatureFlagsManager;
    private flags: FeatureFlags;
    private readonly STORAGE_KEY = 'goose-feature-flags';

    private constructor() {
        // Define default flags
        const defaultFlags: FeatureFlags = {
            whatCanGooseDoText: "What can goose do?",
            expandedToolsByDefault: false,
        };

        // Load flags from storage
        const savedFlags = this.loadFlags();
        
        // Create a new flags object starting with default values
        this.flags = { ...defaultFlags };
        
        // Only override with saved values for keys that exist in default flags
        Object.keys(defaultFlags).forEach((key) => {
            const typedKey = key as keyof FeatureFlags;
            if (savedFlags.hasOwnProperty(key)) {
                this.flags[typedKey] = savedFlags[typedKey];
            }
        });
    }

    private loadFlags(): Partial<FeatureFlags> {
        try {
            const saved = localStorage.getItem(this.STORAGE_KEY);
            return saved ? JSON.parse(saved) : {};
        } catch {
            return {};
        }
    }

    private saveFlags(): void {
        try {
            localStorage.setItem(this.STORAGE_KEY, JSON.stringify(this.flags));
        } catch (error) {
            console.error('Failed to save feature flags:', error);
        }
    }

    public static getInstance(): FeatureFlagsManager {
        if (!FeatureFlagsManager.instance) {
            FeatureFlagsManager.instance = new FeatureFlagsManager();
        }
        return FeatureFlagsManager.instance;
    }

    public getFlags(): FeatureFlags {
        return this.flags;
    }

    public updateFlag<K extends keyof FeatureFlags>(key: K, value: FeatureFlags[K]): void {
        this.flags[key] = value;
        this.saveFlags();
    }
}

export const featureFlags = FeatureFlagsManager.getInstance();
export type { FeatureFlags };