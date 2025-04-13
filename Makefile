.PHONY: dev db

db:
	touch ${DATABASE_URL}
	./db/sqlx.sh migrate

db-revert:
	./db/sqlx.sh revert

db-new:
	./db/sqlx.sh new $(name)

dev:
	cd server && cargo run

prod:
	cargo build --release
	cd frontend && yarn && yarn build && cd ..

setup-python:
	# virtual env
	python3 -m venv .venv
	# install dependencies
	./.venv/bin/pip install -r python/requirements.txt