-- LinkUp başlangıç şeması (PLAN.md §2.12).
--
-- Not: Faz 1'de yalnızca `settings` tablosu fiilen kullanılıyor. Diğer tablolar
-- şemayı baştan tek parça hâlinde kurmak için burada: sonraki fazlar (pairing,
-- chat, transfer, sync) kendi tablolarını eklemek yerine hazır bulacak.

CREATE TABLE trusted_devices (
    id          INTEGER PRIMARY KEY,
    device_id   BLOB    NOT NULL UNIQUE,   -- 32 byte Ed25519 public key
    name        TEXT    NOT NULL,
    alias       TEXT,
    color       TEXT,
    last_ip     TEXT,
    last_port   INTEGER,
    last_seen   INTEGER,
    paired_at   INTEGER NOT NULL
);

CREATE TABLE messages (
    id              INTEGER PRIMARY KEY,
    msg_id          TEXT    NOT NULL UNIQUE,  -- UUID, iki uçta aynı
    conversation_id BLOB    NOT NULL,         -- v1: device_id; v2: grup id
    device_id       BLOB    NOT NULL REFERENCES trusted_devices(device_id) ON DELETE CASCADE,
    direction       TEXT    NOT NULL CHECK (direction IN ('in', 'out')),
    content_type    TEXT    NOT NULL CHECK (content_type IN ('text', 'image', 'code', 'file_ref')),
    content         TEXT    NOT NULL,
    transfer_id     TEXT,
    sent_at         INTEGER NOT NULL,
    status          TEXT    NOT NULL CHECK (status IN ('sending', 'sent', 'delivered', 'read', 'failed'))
);

CREATE INDEX idx_messages_conversation ON messages (conversation_id, sent_at DESC);

-- Tam metin arama (PLAN.md §2.8). `content='messages'` ile external content
-- modunda çalışır: metin iki kez saklanmaz, indeks trigger'larla senkron tutulur.
CREATE VIRTUAL TABLE messages_fts USING fts5 (
    content,
    content='messages',
    content_rowid='id',
    tokenize='unicode61 remove_diacritics 2'
);

CREATE TRIGGER messages_fts_insert AFTER INSERT ON messages BEGIN
    INSERT INTO messages_fts (rowid, content) VALUES (new.id, new.content);
END;

CREATE TRIGGER messages_fts_delete AFTER DELETE ON messages BEGIN
    INSERT INTO messages_fts (messages_fts, rowid, content) VALUES ('delete', old.id, old.content);
END;

CREATE TRIGGER messages_fts_update AFTER UPDATE ON messages BEGIN
    INSERT INTO messages_fts (messages_fts, rowid, content) VALUES ('delete', old.id, old.content);
    INSERT INTO messages_fts (rowid, content) VALUES (new.id, new.content);
END;

CREATE TABLE transfers (
    id            INTEGER PRIMARY KEY,
    transfer_id   TEXT    NOT NULL UNIQUE,
    device_id     BLOB    NOT NULL,
    direction     TEXT    NOT NULL CHECK (direction IN ('in', 'out')),
    file_name     TEXT    NOT NULL,
    file_size     INTEGER NOT NULL,
    mime          TEXT,
    expected_hash BLOB,
    save_path     TEXT,
    part_path     TEXT,
    bytes_done    INTEGER NOT NULL DEFAULT 0,
    status        TEXT    NOT NULL CHECK (status IN ('pending', 'active', 'paused', 'done', 'failed', 'cancelled')),
    error         TEXT,
    started_at    INTEGER NOT NULL,
    completed_at  INTEGER
);

CREATE INDEX idx_transfers_device ON transfers (device_id, started_at DESC);
CREATE INDEX idx_transfers_status ON transfers (status);

CREATE TABLE synced_folders (
    id              INTEGER PRIMARY KEY,
    device_id       BLOB    NOT NULL REFERENCES trusted_devices(device_id) ON DELETE CASCADE,
    local_path      TEXT    NOT NULL,
    remote_path     TEXT    NOT NULL,
    ignore_patterns TEXT,
    enabled         INTEGER NOT NULL DEFAULT 1,
    last_synced_at  INTEGER
);

CREATE TABLE settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
