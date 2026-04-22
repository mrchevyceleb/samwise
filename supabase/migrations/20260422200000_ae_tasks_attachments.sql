-- Add attachments column so tasks can carry image (and other) files that Sam
-- will pass into Claude Code as additional context. Each entry is a JSON
-- object: { "url": "https://…", "name": "bug.png", "mime": "image/png" }.
alter table public.ae_tasks
  add column if not exists attachments jsonb not null default '[]'::jsonb;

-- Storage bucket for Sam task attachments. Public-read so the worker and
-- the web board can fetch by URL without juggling signed URLs; path prefix
-- is a random UUID per task so enumeration is impractical.
insert into storage.buckets (id, name, public)
values ('task-attachments', 'task-attachments', true)
on conflict (id) do update set public = excluded.public;

-- Policies: anyone authenticated or service-role may upload; everyone can
-- read (because public = true already makes GETs work, but explicit select
-- policy keeps listing consistent).
drop policy if exists "task-attachments read" on storage.objects;
create policy "task-attachments read"
on storage.objects for select
to public
using (bucket_id = 'task-attachments');

drop policy if exists "task-attachments insert" on storage.objects;
create policy "task-attachments insert"
on storage.objects for insert
to public
with check (bucket_id = 'task-attachments');
