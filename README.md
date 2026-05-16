docker run -d   --name pcloud-postgres   -e POSTGRES_USER=server   -e POSTGRES_PASSWORD=pcloud   -e POSTGRES_DB=pcloud_db   -p 5432:5432   -v pcloud_pgdata:/var/lib/postgresql/data   postgres:17

## Run in production on this server

1. Confirm `.env` points at the existing PostgreSQL instance and has `PCLOUD_MODE=PROD`.
2. Build and restart the app with `./scripts/deploy-prod.sh`.
3. Tail logs with `journalctl --user -u pcloud-server.service -f`.

The checked-in user service lives at `systemd/user/pcloud-server.service`.

## LAN Discovery

The server advertises itself on the local network with mDNS/DNS-SD by default.
Mobile clients can browse:

```text
_pcloud._tcp.local.
```

The service port is the client web/API port. TXT records include:

```text
version=1
api_path=/api/client
protocol=http
device_id=<PCLOUD_DEVICE_ID>
relay_path=/d/<PCLOUD_DEVICE_ID>
relay_base_url=<PCLOUD_RELAY_PUBLIC_BASE_URL or derived relay URL>
```

Useful environment values:

```env
PCLOUD_MDNS_ENABLED=true
PCLOUD_MDNS_NAME=PCloud My Device
PCLOUD_MDNS_HOSTNAME=pcloud-my-device
PCLOUD_RELAY_PUBLIC_BASE_URL=https://relay.example.com
```

Set `PCLOUD_MDNS_ENABLED=false` to disable LAN discovery. mDNS failures are
logged but do not stop the main HTTP server.
