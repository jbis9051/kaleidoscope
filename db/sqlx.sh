#!/usr/bin/env bash

set -e

echo "Running sqlx.sh"
echo "DATABASE_URL: ${DATABASE_URL}"
pwd

if [ "$1" == "migrate" ]; then
  echo "Migrating database"
  sqlx migrate run --source db/migrations --database-url "sqlite://${DATABASE_URL}"
  exit 0
elif [ "$1" == "revert" ]; then
  echo "Reverting database"
  sqlx migrate revert --source db/migrations --database-url "sqlite://${DATABASE_URL}"
  exit 0
else
  echo "Usage: $0 [migrate|revert]"
  exit 1
fi
