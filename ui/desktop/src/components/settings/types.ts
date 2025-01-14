export interface Model {
    id: string;
    name: string;
    description: string;
    enabled: boolean;
}

export interface Extension {
    id: string;
    name: string;
    description: string;
    enabled: boolean;
}

export interface Key {
    id: string;
    name: string;
    value: string;
}

export interface Settings {
    models: Model[];
    extensions: Extension[];
    keys: Key[];
} 