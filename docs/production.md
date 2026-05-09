# Production runbook

This server can run `pcloud-server` as a `systemd --user` service without changing the existing PostgreSQL setup.

## Current assumptions

- Repo path: `/home/altai/Documents/diploma/pcloud-server`
- Runtime user: `altai`
- Database stays at the value already defined in `.env`
- Client server binds to `0.0.0.0:8080`
- Admin setup server binds to `127.0.0.1:9090` until the system is initialized

## First-time install

```bash
cd /home/altai/Documents/diploma/pcloud-server
./scripts/deploy-prod.sh
```

## Daily operations

```bash
systemctl --user status pcloud-server.service
systemctl --user restart pcloud-server.service
journalctl --user -u pcloud-server.service -f
```

## Important limitation

`loginctl show-user altai` currently reports `Linger=no`.

That means the user service runs correctly while this user has an active session, but it will not automatically survive a reboot or a fully logged-out machine. To make it boot persistently, run this once as root:

```bash
sudo loginctl enable-linger altai
```

If you want a true system-wide service instead, the unit can be adapted into `/etc/systemd/system/`, but that requires root access.
