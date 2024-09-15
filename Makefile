.PHONY: dev db

db:
	touch ${DATABASE_URL}
	./db/sqlx.sh migrate

dev:
	cd server && cargo run

prod:
	cd server && cargo build --release && cd ..
	cd frontend && yarn && yarn build && cd ..