import type { AeComment, AeTask } from '$lib/types';

export const UI_STAMP_KEY = 'samwise_ui_stamp';
export const MANUAL_IN_PROGRESS_STAMP = 'manual_in_progress';
const LEGACY_COPILOT_REVIEW_STAMP = 'copilot_review';
export const MERGE_DEPLOY_REQUESTED_AT_KEY = 'samwise_merge_deploy_requested_at';
export const MERGE_DEPLOY_STARTED_AT_KEY = 'samwise_merge_deploy_started_at';
export const MERGE_DEPLOY_STATUS_KEY = 'samwise_merge_deploy_status';
export const MERGE_DEPLOY_ERROR_KEY = 'samwise_merge_deploy_error';

export type ReviewVerdict = 'merge' | 'fix' | 'inconclusive' | 'errored' | 'blocked';

export interface ReviewActionPanel {
	verdict: ReviewVerdict;
	label: string;
	why: string;
	deployment: string;
	hasDeploymentCallout: boolean;
	source: 'samwise-pr-review' | 'auto-merge' | 'error';
}

export type MergeDeployStatus = 'requested' | 'running' | 'succeeded' | 'failed';

export interface MergeDeployState {
	status: MergeDeployStatus | null;
	requestedAt: string | null;
	startedAt: string | null;
	error: string | null;
}

export function getUiStamp(task: Pick<AeTask, 'context'>): typeof MANUAL_IN_PROGRESS_STAMP | null {
	const stamp = task.context?.[UI_STAMP_KEY];
	return stamp === MANUAL_IN_PROGRESS_STAMP || stamp === LEGACY_COPILOT_REVIEW_STAMP ? MANUAL_IN_PROGRESS_STAMP : null;
}

export function nextManualInProgressStampContext(task: Pick<AeTask, 'context'>): Record<string, unknown> | null {
	const context = { ...(task.context ?? {}) };
	if (getUiStamp(task)) {
		delete context[UI_STAMP_KEY];
		return Object.keys(context).length > 0 ? context : null;
	}
	context[UI_STAMP_KEY] = MANUAL_IN_PROGRESS_STAMP;
	return context;
}

export function getMergeDeployState(task: Pick<AeTask, 'context'>): MergeDeployState {
	const context = task.context ?? {};
	const rawStatus = context[MERGE_DEPLOY_STATUS_KEY];
	const status: MergeDeployStatus | null =
		rawStatus === 'requested' || rawStatus === 'running' || rawStatus === 'succeeded' || rawStatus === 'failed'
			? rawStatus
			: null;
	return {
		status,
		requestedAt: stringValue(context[MERGE_DEPLOY_REQUESTED_AT_KEY]),
		startedAt: stringValue(context[MERGE_DEPLOY_STARTED_AT_KEY]),
		error: stringValue(context[MERGE_DEPLOY_ERROR_KEY]),
	};
}

export function requestMergeDeployContext(task: Pick<AeTask, 'context'>): Record<string, unknown> {
	return {
		...(task.context ?? {}),
		[MERGE_DEPLOY_REQUESTED_AT_KEY]: new Date().toISOString(),
		[MERGE_DEPLOY_STATUS_KEY]: 'requested',
		[MERGE_DEPLOY_ERROR_KEY]: null,
	};
}

export function mergeDeployButtonLabel(state: MergeDeployState): string {
	if (state.status === 'running') return 'Deploying...';
	if (state.status === 'requested') return 'Merge Queued';
	if (state.status === 'failed') return 'Retry Merge + Deploy';
	return 'Merge + Deploy';
}

export function isMergeDeployBusy(state: MergeDeployState): boolean {
	return state.status === 'requested' || state.status === 'running';
}

export function isReviewActionStatus(status: AeTask['status']): boolean {
	return status === 'review' || status === 'fixes_needed' || status === 'approved' || status === 'done';
}

export function extractReviewActionPanel(task: AeTask, comments: AeComment[]): ReviewActionPanel | null {
	const sorted = [...comments].sort((a, b) => a.created_at.localeCompare(b.created_at));
	const reviewCommentIndex = findLatestReviewHeadline(sorted);

	if (reviewCommentIndex >= 0) {
		const headline = sorted[reviewCommentIndex].content;
		const body = findReviewBody(sorted, reviewCommentIndex);
		const verdict = verdictFromHeadline(headline);
		const parsed = panelFromBody(verdict, body || headline, task);
		return { ...parsed, source: verdict === 'errored' ? 'error' : 'samwise-pr-review' };
	}

	if (task.review_summary || task.auto_merge_blocked_reason) {
		const verdict: ReviewVerdict = task.auto_merge_blocked_reason ? 'blocked' : 'merge';
		const deployment = extractDeploymentRequirement('', task);
		return {
			verdict,
			label: verdictLabel(verdict),
			why: cleanInline(task.auto_merge_blocked_reason || task.review_summary || 'Auto-merge review passed.'),
			deployment: deployment.text,
			hasDeploymentCallout: deployment.calledOut,
			source: 'auto-merge',
		};
	}

	return null;
}

function findLatestReviewHeadline(comments: AeComment[]): number {
	for (let i = comments.length - 1; i >= 0; i -= 1) {
		const content = comments[i].content;
		if (/Codex says:\s*\*\*(MERGE|FIX|INCONCLUSIVE)\*\*/i.test(content)) return i;
		if (/Codex review errored:/i.test(content)) return i;
	}
	return -1;
}

function findReviewBody(comments: AeComment[], headlineIndex: number): string {
	for (let i = headlineIndex + 1; i < comments.length; i += 1) {
		if (looksLikeReviewBody(comments[i].content)) return comments[i].content;
	}
	for (let i = headlineIndex - 1; i >= 0; i -= 1) {
		if (looksLikeReviewBody(comments[i].content)) return comments[i].content;
	}
	return '';
}

function looksLikeReviewBody(content: string): boolean {
	return /##\s+(Summary|Blockers|Risks|Not verified|Deployment Required)/i.test(content)
		|| /^VERDICT:\s*(MERGE|FIX|INCONCLUSIVE)/im.test(content);
}

function verdictFromHeadline(content: string): ReviewVerdict {
	if (/Codex review errored:/i.test(content)) return 'errored';
	const match = content.match(/Codex says:\s*\*\*(MERGE|FIX|INCONCLUSIVE)\*\*/i);
	const raw = match?.[1]?.toLowerCase();
	if (raw === 'merge') return 'merge';
	if (raw === 'fix') return 'fix';
	return 'inconclusive';
}

function panelFromBody(verdict: ReviewVerdict, body: string, task: AeTask): ReviewActionPanel {
	const summary = firstUsefulLine(section(body, 'Summary')) || firstUsefulLine(task.review_summary || '');
	const blockers = cleanedSectionLines(section(body, 'Blockers'));
	const risks = cleanedSectionLines(section(body, 'Risks'));
	const deployment = extractDeploymentRequirement(body, task);

	let why = summary || 'Review completed.';
	if (verdict === 'merge') {
		why = summary ? `No blockers. ${summary}` : 'Codex found no merge-blocking issues.';
	} else if (verdict === 'fix' || verdict === 'blocked') {
		why = blockers.length > 0 ? blockers.slice(0, 2).join(' ') : (risks[0] || summary || 'Codex found issues that need fixes before merge.');
	} else if (verdict === 'inconclusive') {
		why = risks[0] || summary || 'Codex could not verify this PR enough to recommend a merge.';
	} else if (verdict === 'errored') {
		why = cleanInline(body.replace(/^Codex review errored:\s*/i, 'Codex review errored: '));
	}

	return {
		verdict,
		label: verdictLabel(verdict),
		why: clampText(why, 220),
		deployment: deployment.text,
		hasDeploymentCallout: deployment.calledOut,
		source: 'samwise-pr-review',
	};
}

function verdictLabel(verdict: ReviewVerdict): string {
	if (verdict === 'merge') return 'Codex: merge';
	if (verdict === 'fix') return 'Codex: fix first';
	if (verdict === 'blocked') return 'Auto-merge blocked';
	if (verdict === 'errored') return 'Codex errored';
	return 'Codex: human check';
}

function section(markdown: string, heading: string): string {
	const lines = markdown.split(/\r?\n/);
	const headingRe = new RegExp(`^##\\s+${escapeRegExp(heading)}\\s*$`, 'i');
	const nextHeadingRe = /^##\s+/;
	const start = lines.findIndex((line) => headingRe.test(line.trim()));
	if (start < 0) return '';
	const collected: string[] = [];
	for (let i = start + 1; i < lines.length; i += 1) {
		if (nextHeadingRe.test(lines[i].trim())) break;
		collected.push(lines[i]);
	}
	return collected.join('\n').trim();
}

function extractDeploymentRequirement(body: string, task: AeTask): { text: string; calledOut: boolean } {
	const explicit = section(body, 'Deployment Required') || section(body, 'Deployment');
	const explicitLines = cleanedSectionLines(explicit);
	if (explicitLines.length > 0) {
		return { text: explicitLines.slice(0, 3).join(' '), calledOut: true };
	}

	const haystack = `${body}\n${task.title}\n${task.description || ''}`.toLowerCase();
	const items: string[] = [];
	if (/\brailway\b|railway server|server deploy|backend deploy/.test(haystack)) {
		items.push('Railway server deploy likely required.');
	}
	if (/supabase\/migrations|supabase migration|\bmigrations?\b|database migration|schema change/.test(haystack)) {
		items.push('Supabase migration deploy likely required.');
	}
	if (/supabase\/functions|edge function|edge functions|functions\/|supabase functions/.test(haystack)) {
		items.push('Supabase Edge Function deploy likely required.');
	}

	return {
		text: items.length > 0 ? items.join(' ') : 'No Railway/Supabase deploy requirement called out.',
		calledOut: items.length > 0,
	};
}

function cleanedSectionLines(value: string): string[] {
	return value
		.split(/\r?\n/)
		.map((line) => line.replace(/^\s*[-*]\s*/, '').trim())
		.filter((line) => line.length > 0)
		.filter((line) => !/^<none>|none\.?$/i.test(line))
		.map(cleanInline);
}

function firstUsefulLine(value: string): string {
	const line = cleanedSectionLines(value)[0] || '';
	return cleanInline(line.split(/(?<=[.!?])\s+/)[0] || line);
}

function cleanInline(value: string): string {
	return value
		.replace(/\*\*/g, '')
		.replace(/`/g, '')
		.replace(/\s+/g, ' ')
		.trim();
}

function clampText(value: string, max: number): string {
	const clean = cleanInline(value);
	return clean.length > max ? `${clean.slice(0, max - 1).trim()}...` : clean;
}

function escapeRegExp(value: string): string {
	return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

function stringValue(value: unknown): string | null {
	return typeof value === 'string' && value.trim().length > 0 ? value : null;
}
