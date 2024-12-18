.PHONY: dev db

db:
	touch ${DATABASE_URL}
	./db/sqlx.sh migrate

dev:
	cd server && cargo run

prod:
	cargo build --release
	cd frontend && yarn && yarn build && cd ..