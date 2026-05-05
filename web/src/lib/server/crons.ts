import { error } from '@sveltejs/kit';

const CRON_FIELD_COUNTS = new Set([5, 6, 7]);
const UPDATE_FIELDS = new Set(['name', 'schedule', 'task_template', 'enabled', 'next_run']);

function asRecord(value: unknown): Record<string, unknown> {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    throw error(400, 'JSON object required');
  }
  return value as Record<string, unknown>;
}

function cleanName(value: unknown): string {
  if (typeof value !== 'string') throw error(400, 'name must be a string');
  const name = value.trim();
  if (!name) throw error(400, 'name is required');
  if (name.length > 200) throw error(400, 'name is too long');
  return name;
}

function cleanSchedule(value: unknown): string {
  if (typeof value !== 'string') throw error(400, 'schedule must be a string');
  const schedule = value.trim().replace(/\s+/g, ' ');
  if (!schedule) throw error(400, 'schedule is required');

  const fields = schedule.split(' ');
  if (!CRON_FIELD_COUNTS.has(fields.length)) {
    throw error(400, 'schedule must have 5, 6, or 7 cron fields');
  }
  if (fields.some((field) => field.length > 64)) {
    throw error(400, 'schedule contains an invalid field');
  }
  return schedule;
}

function parseCronField(field: string, min: number, max: number, sundaySeven = false): Set<number> {
  const values = new Set<number>();
  const addRange = (start: number, end: number, step = 1) => {
    if (!Number.isInteger(start) || !Number.isInteger(end) || !Number.isInteger(step) || step < 1) {
      throw error(400, 'schedule contains an invalid field');
    }
    if (sundaySeven && end === 7) end = 0;
    if (sundaySeven && start === 7) start = 0;
    if (start < min || start > max || end < min || end > max) {
      throw error(400, 'schedule contains an out-of-range field');
    }
    if (start <= end) {
      for (let value = start; value <= end; value += step) values.add(value);
    } else if (sundaySeven) {
      for (let value = start; value <= max; value += step) values.add(value);
      for (let value = min; value <= end; value += step) values.add(value);
    } else {
      throw error(400, 'schedule contains an invalid range');
    }
  };

  for (const part of field.split(',')) {
    const [base, stepText] = part.split('/');
    const step = stepText === undefined ? 1 : Number(stepText);
    if (base === '*') {
      addRange(min, max, step);
      continue;
    }

    if (base.includes('-')) {
      const [start, end] = base.split('-').map(Number);
      addRange(start, end, step);
      continue;
    }

    const value = Number(base);
    addRange(value, value, step);
  }

  return values;
}

export function nextRunForSchedule(schedule: string, from = new Date()): string {
  const fields = cleanSchedule(schedule).split(' ');
  const nowYear = from.getUTCFullYear();
  const [minuteField, hourField, domField, monthField, dowField, yearField] =
    fields.length === 7 ? fields.slice(1) : fields;

  const minutes = parseCronField(minuteField, 0, 59);
  const hours = parseCronField(hourField, 0, 23);
  const days = parseCronField(domField, 1, 31);
  const months = parseCronField(monthField, 1, 12);
  const weekdays = parseCronField(dowField, 0, 7, true);
  const years = yearField ? parseCronField(yearField, nowYear, nowYear + 5) : null;

  const candidate = new Date(from);
  candidate.setUTCSeconds(0, 0);
  candidate.setUTCMinutes(candidate.getUTCMinutes() + 1);

  const maxMinutes = 60 * 24 * 366 * 5;
  for (let i = 0; i < maxMinutes; i += 1) {
    if (
      minutes.has(candidate.getUTCMinutes()) &&
      hours.has(candidate.getUTCHours()) &&
      days.has(candidate.getUTCDate()) &&
      months.has(candidate.getUTCMonth() + 1) &&
      weekdays.has(candidate.getUTCDay()) &&
      (!years || years.has(candidate.getUTCFullYear()))
    ) {
      return candidate.toISOString();
    }
    candidate.setUTCMinutes(candidate.getUTCMinutes() + 1);
  }

  throw error(400, 'schedule does not produce a run time in the next 5 years');
}

function cleanTemplate(value: unknown): Record<string, unknown> {
  const template = asRecord(value);
  const hasText = (candidate: unknown) => typeof candidate === 'string' && candidate.trim().length > 0;
  const hasTarget =
    hasText(template.project) ||
    hasText(template.repo_parent) ||
    hasText(template.repo_url) ||
    hasText(template.repo_path);

  if (!hasTarget) {
    throw error(400, 'task_template must include project, repo_parent, repo_url, or repo_path');
  }

  return template;
}

export function normalizeCronInsert(value: unknown): Record<string, unknown> {
  const body = asRecord(value);
  if (body.enabled !== undefined && typeof body.enabled !== 'boolean') {
    throw error(400, 'enabled must be a boolean');
  }

  const schedule = cleanSchedule(body.schedule);
  const enabled = body.enabled !== false;
  return {
    name: cleanName(body.name),
    schedule,
    task_template: cleanTemplate(body.task_template ?? {}),
    enabled,
    next_run: typeof body.next_run === 'string'
      ? body.next_run
      : enabled
        ? nextRunForSchedule(schedule)
        : null
  };
}

export function normalizeCronUpdate(value: unknown): Record<string, unknown> {
  const body = asRecord(value);
  const updates: Record<string, unknown> = {};

  for (const key of Object.keys(body)) {
    if (!UPDATE_FIELDS.has(key)) continue;

    if (key === 'name') updates.name = cleanName(body.name);
    if (key === 'schedule') updates.schedule = cleanSchedule(body.schedule);
    if (key === 'task_template') updates.task_template = cleanTemplate(body.task_template);
    if (key === 'enabled') {
      if (typeof body.enabled !== 'boolean') throw error(400, 'enabled must be a boolean');
      updates.enabled = body.enabled;
    }
    if (key === 'next_run') {
      if (typeof body.next_run !== 'string' && body.next_run !== null) {
        throw error(400, 'next_run must be a string or null');
      }
      updates.next_run = body.next_run;
    }
  }

  if (Object.keys(updates).length === 0) {
    throw error(400, 'No valid cron fields to update');
  }

  return updates;
}
