export type Prompt = {
	id: string;
	name: string;
	createdBy: string;
	createdAt: Date;
};

export type PromptStatus = "PUBLISHED" | "DRAFT";

export interface PromptInput {
	name: string;
	prompt: string;
	version: string;
	status: PromptStatus;
	tags: string[];
	metaProperties: Record<string, unknown>;
}

export interface PromptUpdate {
	versionId?: string;
	promptId: string;
	prompt?: string;
	version?: string;
	status?: PromptStatus;
	tags?: string[];
	metaProperties?: Record<string, unknown>;
}

export type PromptVersion = {
	id: string;
	promptId: string;
	version: string;
	updatedBy: string;
	updatedAt: Date;
	prompt: string;
	status: PromptStatus;
	tags: string;
	metaProperties: Record<string, unknown>;
};

export type PromptCompiledInput = {
	id?: string;
	name?: string;
	apiKey: string;
	variables?: Record<string, any>;
	version?: string;
	shouldCompile?: boolean;
	downloadSource?: string;
	downloadMetaProperties?: Record<string, unknown>;
};

export type SpecificPromptInput = {
	id?: string;
	name?: string;
	version?: string;
};

export type PromptDownloadInput = {
	promptId: string;
	versionId: string;
	downloadSource: string;
	metaProperties?: Record<string, unknown>;
};
