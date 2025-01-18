-- media info: cached metadata about tracked media files
CREATE TABLE IF NOT EXISTS info(
    id TEXT NOT NULL PRIMARY KEY,
    -- note: this would preferably be unique, but that messes with modern sqlite
    --       update-insert syntax...
    path TEXT NOT NULL,
    filesize INTEGER NOT NULL,
    format TEXT NOT NULL,
    creation_date DATETIME,
    modification_date DATETIME,
    first_seen_date DATETIME NOT NULL,
    width_px INTEGER NOT NULL,
    height_px INTEGER NOT NULL,
    specific_metadata TEXT NOT NULL,
    other_metadata TEXT,
    tags TEXT NOT NULL
);

-- thumbnails: preview media
CREATE TABLE IF NOT EXISTS thumbnail(
    -- path to the thumbnail on disk
    path TEXT NOT NULL,
    -- thumbnail is for the media file with this uuid
    --
    -- TODO: migrate to `media_id`
    media_id TEXT NOT NULL PRIMARY KEY
);

-- albums: contain media
CREATE TABLE IF NOT EXISTS album(
    id TEXT NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    path TEXT NOT NULL,
    --
    -- uuids (in json)
    contained_media TEXT NOT NULL
);

-- hashes: media file hashes to ensure metadata is up-to-date!
CREATE TABLE IF NOT EXISTS hashes(
    media_id TEXT NOT NULL PRIMARY KEY,
    hash BLOB NOT NULL
);

-- hash_blob_index: tell SQLite to make a btree for the hashes, too.
--
-- (this allows for high-speed lookups, both ways. hash <=> id)
CREATE UNIQUE INDEX IF NOT EXISTS hash_blob_index ON hashes(hash);