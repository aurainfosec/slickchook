-- Adds target-scoped run lookup and widens the status CHECK constraint to
-- include 'error', which record_run has always been able to write but which
-- the original constraint rejected (silently dropping those runs).
ALTER TABLE plugin_runs DROP CONSTRAINT IF EXISTS plugin_runs_status_check;
ALTER TABLE plugin_runs ADD CONSTRAINT plugin_runs_status_check
  CHECK (status IN ('running', 'success', 'failed', 'error'));

ALTER TABLE plugin_runs ADD COLUMN IF NOT EXISTS target TEXT;
CREATE INDEX IF NOT EXISTS plugin_runs_target_idx ON plugin_runs (target);
