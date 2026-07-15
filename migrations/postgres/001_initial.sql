-- Slickchook PostgreSQL schema. Apply with a migration tool, not manually.
CREATE TABLE IF NOT EXISTS plugin_runs (
    run_id UUID PRIMARY KEY,
    plugin TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('running', 'success', 'failed')),
    exit_code INTEGER,
    input_json JSONB NOT NULL,
    artifact_dir TEXT NOT NULL,
    stdout_artifact TEXT NOT NULL,
    stderr_artifact TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS plugin_runs_status_idx ON plugin_runs (status);
CREATE INDEX IF NOT EXISTS plugin_runs_created_idx ON plugin_runs (created_at);

CREATE TABLE IF NOT EXISTS mcp_sessions (
    session_id UUID PRIMARY KEY,
    subject TEXT NOT NULL,
    client_id TEXT NOT NULL,
    protocol_version TEXT NOT NULL,
    state_json JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX IF NOT EXISTS mcp_sessions_subject_idx ON mcp_sessions (subject);

CREATE TABLE IF NOT EXISTS mcp_events (
    session_id UUID NOT NULL REFERENCES mcp_sessions(session_id) ON DELETE CASCADE,
    event_id BIGINT GENERATED ALWAYS AS IDENTITY,
    event_type TEXT NOT NULL,
    payload_json JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (session_id, event_id)
);
