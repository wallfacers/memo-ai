-- memo-ai SQLite schema
-- Loaded at runtime via include_str! macro

CREATE TABLE IF NOT EXISTS meetings (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    title       TEXT    NOT NULL,
    start_time  TEXT    NOT NULL,
    end_time    TEXT,
    status      TEXT    NOT NULL DEFAULT 'idle',
    summary     TEXT,
    report      TEXT,
    audio_path  TEXT,
    created_at  TEXT    NOT NULL,
    updated_at  TEXT    NOT NULL
);

CREATE TABLE IF NOT EXISTS transcripts (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    meeting_id  INTEGER NOT NULL REFERENCES meetings(id) ON DELETE CASCADE,
    speaker     TEXT,
    text        TEXT    NOT NULL,
    timestamp   REAL    NOT NULL DEFAULT 0,
    confidence  REAL,
    created_at  TEXT    NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_transcripts_meeting_id ON transcripts(meeting_id);

CREATE TABLE IF NOT EXISTS action_items (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    meeting_id  INTEGER NOT NULL REFERENCES meetings(id) ON DELETE CASCADE,
    task        TEXT    NOT NULL,
    owner       TEXT,
    deadline    TEXT,
    status      TEXT    NOT NULL DEFAULT 'pending',
    created_at  TEXT    NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_action_items_meeting_id ON action_items(meeting_id);

CREATE TABLE IF NOT EXISTS meeting_structures (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    meeting_id   INTEGER NOT NULL UNIQUE REFERENCES meetings(id) ON DELETE CASCADE,
    topic        TEXT,
    participants TEXT    NOT NULL DEFAULT '[]',
    key_points   TEXT    NOT NULL DEFAULT '[]',
    decisions    TEXT    NOT NULL DEFAULT '[]',
    risks        TEXT    NOT NULL DEFAULT '[]',
    created_at   TEXT    NOT NULL
);
