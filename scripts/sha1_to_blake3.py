#!/usr/bin/env python3

"""
Converts all SHA1 hashes in the media table to BLAKE3 hashes
"""
import sqlite3
import sys
from blake3 import blake3

if len(sys.argv) != 2:
    print("Usage: python3 sha1_to_blake3.py <database_url>")
    sys.exit(1)

db_url = sys.argv[1]

db = sqlite3.connect(db_url)

cur = db.cursor()

# Get all the SHA1 hashes from the media table
res = cur.execute("SELECT media.id, media.name, media.path, media.hash FROM media WHERE LENGTH(media.hash) = 40")
rows = res.fetchall()
count = len(rows)

if count == 0:
    print("No SHA1 hashes found")
    sys.exit(0)

# Confirm they want to convert the hashes
res = input(f"Found {count} SHA1 hashes. Consider making a backup first. Confirm conversion? (y/n) ")

if res.lower() != "y":
    print("Aborting")
    sys.exit(0)

for i, row in enumerate(rows):
    id, name, path, hash = row
    # Feed the file into a blake3 hash
    blake_digest = None
    with open(path, "rb") as f:
        hasher = blake3()
        for chunk in iter(lambda: f.read(8192), b''):
            hasher.update(chunk)
        blake_digest = hasher.hexdigest()
    # Update the database with the new hash
    cur.execute("UPDATE media SET hash = ? WHERE id = ?", (blake_digest, id))
    db.commit()
    print(f"{i + 1}/{count} - {name} - {path} - {hash} -> {blake_digest}")

print("Conversion complete: {count} SHA1 hashes converted to BLAKE3")