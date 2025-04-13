#!/usr/bin/env python3

"""
This script moves all the thumbnails and full images from the current directory into a subdirectory structure based on the first two characters of their UUIDs: thumbnail/a/b
"""

import os
import re
import shutil
import sys
from pathlib import Path


# usage: thumbnail-subdir.py <thumbnail_dir>

if len(sys.argv) != 2:
    print("Usage: python3 thumbnail-subdir.py <thumbnail_dir>")
    sys.exit(1)

thumbnail_dir = sys.argv[1]

# Regex to match <uuid>-thumb.jpg or <uuid>-full.jpg
pattern = re.compile(r'^([0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12})-(thumb|full)\.jpg$', re.IGNORECASE)


to_move = []

for filename in os.listdir(thumbnail_dir):
    match = pattern.match(filename)
    if not match:
        # Skip files that do not match the pattern
        continue
    to_move.append(filename)


# Confirm they want to move the files
if len(to_move) == 0:
    print("No files to move")
    sys.exit(0)

res = input(f"Found {len(to_move)} files. Confirm move? (y/n) ")

if res.lower() != 'y':
    print("Aborting")
    sys.exit(0)

# Iterate over all files in the thumbnail directory
moved = 0
for filename in to_move:
    a = filename[0]
    b = filename[1]
    # Create the subdirectory path
    subdir = os.path.join(thumbnail_dir, a, b)
    # Create the subdirectory if it does not exist
    Path(subdir).mkdir(parents=True, exist_ok=True)
    # Move the file to the subdirectory
    src = os.path.join(thumbnail_dir, filename)
    dst = os.path.join(subdir, filename)
    print(f"Moving {src} to {dst}")
    moved += 1
    shutil.move(src, dst)

print("Moved", moved, "files")