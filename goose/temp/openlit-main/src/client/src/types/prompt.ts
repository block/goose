export interface PromptList {
	promptId: string;
	name: string;
	createdBy: string,
	version: string;
	totalVersions: number;
	latestVersion: string;
	latestVersionDate: string;
	latestVersionStatus: PromptVersionStatus,
	totalDownloads: number
}

export interface Prompt {
	promptId: string;
	name: string;
	createdBy: string;
	createdAt: string;
	versionId: string;
	prompt: string;
	status: PromptVersionStatus;
	updatedAt: string;
	version: string;
	metaProperties: string;
	tags: string;
  versions: PromptVersion[]
}

export interface PromptVersion {
	versionId: string;
	promptId: string;
	prompt: string;
	version: string;
	status: PromptVersionStatus;
	metaProperties: string;
	tags: string;
	updatedBy: string;
	updatedAt: string;
	totalDownloads: number;
}

export enum PromptVersionStatus {
	PUBLISHED = "PUBLISHED",
	DRAFT = "DRAFT",
}