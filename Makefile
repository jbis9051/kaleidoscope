.PHONY: dev db

db:
	touch ${DATABASE_URL}
	./db/sqlx.sh migrate

db-revert:
	./db/sqlx.sh revert

dev:
	cd server && cargo run

prod:
	cargo build --release
	cd frontend && yarn && yarn build && cd ..