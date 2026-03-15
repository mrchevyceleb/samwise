const BASE = 'https://api.doppler.com/v3';

export interface DopplerProject {
	slug: string;
	name: string;
}

export interface DopplerConfig {
	name: string;
	environment: string;
}

async function dopplerFetch<T>(token: string, path: string): Promise<T> {
	const res = await fetch(`${BASE}${path}`, {
		headers: { Authorization: `Bearer ${token}` },
	});
	if (!res.ok) {
		let msg = `Doppler API error (${res.status})`;
		try {
			const body = await res.json();
			if (body?.messages?.length) msg = body.messages.join(', ');
		} catch { /* use default msg */ }
		throw new Error(msg);
	}
	return res.json();
}

export async function fetchProjects(token: string): Promise<DopplerProject[]> {
	const data = await dopplerFetch<{ projects: DopplerProject[] }>(token, '/projects?per_page=100');
	return data.projects ?? [];
}

export async function fetchConfigs(token: string, project: string): Promise<DopplerConfig[]> {
	const data = await dopplerFetch<{ configs: DopplerConfig[] }>(
		token,
		`/configs?project=${encodeURIComponent(project)}&per_page=100`,
	);
	return data.configs ?? [];
}

export async function fetchSecrets(
	token: string,
	project: string,
	config: string,
): Promise<Record<string, string>> {
	const data = await dopplerFetch<{ secrets: Record<string, { computed: string }> }>(
		token,
		`/configs/config/secrets?project=${encodeURIComponent(project)}&config=${encodeURIComponent(config)}`,
	);
	const result: Record<string, string> = {};
	for (const [key, val] of Object.entries(data.secrets ?? {})) {
		if (key.startsWith('DOPPLER_')) continue;
		result[key] = val.computed;
	}
	return result;
}
