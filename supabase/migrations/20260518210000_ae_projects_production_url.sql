-- qa-verify staging/production selector: ae_projects gets a production_url.
-- preview_url is the staging/default QA target; production_url is the prod
-- target. Task creation resolves project + environment -> concrete preview_url.
alter table public.ae_projects add column if not exists production_url text;
comment on column public.ae_projects.production_url is
  'Production QA target URL. preview_url is the staging/default; qa-verify picks based on task environment (default staging).';
