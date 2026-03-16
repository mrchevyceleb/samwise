import { invoke } from '@tauri-apps/api/core';

export interface DopplerProject {
	slug: string;
	name: string;
}

export interface DopplerConfig {
	name: string;
	environment: string;
}

export async function fetchProjects(token: string): Promise<DopplerProject[]> {
	return invoke<DopplerProject[]>('doppler_fetch_projects', { token });
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
