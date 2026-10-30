CREATE TABLE repository (id TEXT PRIMARY KEY NOT NULL);
CREATE TABLE blobs (
 id TEXT PRIMARY KEY NOT NULL, owner TEXT NOT NULL,
 size_bytes INTEGER NOT NULL CHECK(size_bytes >= 0), sha256 TEXT NOT NULL,
 state TEXT NOT NULL CHECK(state IN ('writing','ready','deleted'))
);
