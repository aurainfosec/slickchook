-- Supports recording a run the moment it starts (status 'running', no
-- finish time yet) rather than only once it completes, and recording a
-- genuinely cancelled run as such - both needed so a cancelled or
-- crashed run stays visible via list_runs_by_target/purge_runs_by_target
-- instead of leaving an orphaned artifact directory with no database row
-- at all. See SECURITY_REVIEW_2026-10-20.md's R6.
ALTER TABLE plugin_runs ALTER COLUMN finished_at DROP NOT NULL;

ALTER TABLE plugin_runs DROP CONSTRAINT IF EXISTS plugin_runs_status_check;
ALTER TABLE plugin_runs ADD CONSTRAINT plugin_runs_status_check
  CHECK (status IN ('running', 'success', 'failed', 'error', 'cancelled'));
