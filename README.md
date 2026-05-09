docker run -d   --name pcloud-postgres   -e POSTGRES_USER=server   -e POSTGRES_PASSWORD=pcloud   -e POSTGRES_DB=pcloud_db   -p 5432:5432   -v pcloud_pgdata:/var/lib/postgresql/data   postgres:17

## Run in production on this server

1. Confirm `.env` points at the existing PostgreSQL instance and has `PCLOUD_MODE=PROD`.
2. Build and restart the app with `./scripts/deploy-prod.sh`.
3. Tail logs with `journalctl --user -u pcloud-server.service -f`.

The checked-in user service lives at `systemd/user/pcloud-server.service`.
