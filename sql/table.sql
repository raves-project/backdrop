-- uncomment to delete old data
DROP TABLE info;

DROP TABLE thumbnail;

--
CREATE TABLE info(
    id TEXT NOT NULL PRIMARY KEY,
    path TEXT NOT NULL UNIQUE,
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

CREATE TABLE thumbnail(
    path TEXT NOT NULL,
    image_id TEXT NOT NULL PRIMARY KEY
);