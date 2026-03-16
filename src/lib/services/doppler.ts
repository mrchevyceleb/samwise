import { invoke } from '@tauri-apps/api/core';

export interface DopplerWorkplace {
	id?: string;
	name?: string;
}

export interface DopplerProject {
	slug: string;
	name: string;
}

export interface DopplerConfig {
	name: string;
	environment: string;
}

/** A named Doppler token with its auto-detected org info */
export interface DopplerTokenEntry {
	token: string;
	orgName: string;
	orgSlug: string;
}

/** Fetch the organization this token belongs to (each token = one org) */
export async function fetchWorkplace(token: string): Promise<DopplerWorkplace> {
	return invoke<DopplerWorkplace>('doppler_fetch_workplaces', { token });
}

export async function fetchProjects(token: string, workplace?: string): Promise<DopplerProject[]> {
	return invoke<DopplerProject[]>('doppler_fetch_projects', { token, workplace: workplace || null });
}

export async function fetchConfigs(token: string, project: string): Promise<DopplerConfig[]> {
	return invoke<DopplerConfig[]>('doppler_fetch_configs', { token, project });
}

export async function fetchSecrets(
	token: string,
	project: string,
	config: string,
): Promise<Record<string, string>> {
	return invoke<Record<string, string>>('doppler_fetch_secrets', { token, project, config });
}
