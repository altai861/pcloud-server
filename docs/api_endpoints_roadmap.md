# PCloud API Endpoints Roadmap

This file lists the full endpoint set recommended for your current Personal Cloud system.

Status meanings:
- `implemented`: already present in current Rust server
- `planned`: should be implemented next

Base path: `/api`

Auth meanings:
- `public`: no login required
- `user`: requires valid user session token
- `admin`: requires valid admin session token

## 1. Setup and Bootstrap

| Status | Method | Path | Auth | Purpose |
|---|---|---|---|---|
| implemented | GET | `/api/setup/status` | public | Check whether system initialization is completed |
| implemented | POST | `/api/setup/initialize` | public (first run only) | Create first admin and initialize storage settings |
| planned | GET | `/api/setup/requirements` | public | Return setup prerequisites and validation rules |
| planned | POST | `/api/setup/validate-storage-path` | public | Validate storage path before final initialize |

## 2. Client System and Health

| Status | Method | Path | Auth | Purpose |
|---|---|---|---|---|
| implemented | GET | `/api/client/status` | public | Basic server status for client app |
| planned | GET | `/api/client/health` | public | Extended health info (db, storage, version) |
| planned | GET | `/api/client/capabilities` | public | Return enabled features for web/mobile clients |

## 3. Authentication and Session

| Status | Method | Path | Auth | Purpose |
|---|---|---|---|---|
| implemented | POST | `/api/client/auth/login` | public | Login with username and password |
| planned | POST | `/api/client/auth/refresh` | public (refresh token) | Rotate and issue new access token |
| implemented | POST | `/api/client/auth/logout` | user | Revoke current session |
| planned | POST | `/api/client/auth/logout-all` | user | Revoke all sessions for current user |
| implemented | GET | `/api/client/me` | user | Get current user profile |
| planned | PATCH | `/api/client/me` | user | Update own profile fields |
| planned | PATCH | `/api/client/me/password` | user | Change own password |
| planned | GET | `/api/client/sessions` | user | List current user active sessions/devices |
| planned | DELETE | `/api/client/sessions/{sessionId}` | user | Revoke a specific session/device |

## 4. Storage Browser and Search

| Status | Method | Path | Auth | Purpose |
|---|---|---|---|---|
| implemented | GET | `/api/client/storage/list` | user | List folder entries by relative path |
| planned | GET | `/api/client/storage/tree` | user | Return lightweight folder tree |
| planned | GET | `/api/client/storage/usage` | user | Return per-user usage and quota details |
| planned | GET | `/api/client/search` | user | Search files/folders by name and filters |
| planned | GET | `/api/client/recent` | user | Return recent files and folders |
| planned | GET | `/api/client/starred` | user | Return starred items |

## 5. Folder Management

| Status | Method | Path | Auth | Purpose |
|---|---|---|---|---|
| planned | POST | `/api/client/folders` | user | Create folder |
| planned | GET | `/api/client/folders/{folderId}` | user | Get folder metadata and permissions summary |
| planned | PATCH | `/api/client/folders/{folderId}` | user | Rename folder |
| planned | PATCH | `/api/client/folders/{folderId}/move` | user | Move folder to another parent |
| planned | DELETE | `/api/client/folders/{folderId}` | user | Soft-delete folder (to trash) |
| planned | POST | `/api/client/folders/{folderId}/restore` | user | Restore folder from trash |
| planned | DELETE | `/api/client/folders/{folderId}/hard` | user | Permanently delete folder |

## 6. File Management

| Status | Method | Path | Auth | Purpose |
|---|---|---|---|---|
| planned | POST | `/api/client/files/upload` | user | Upload a file (multipart or stream) |
| planned | GET | `/api/client/files/{fileId}` | user | Get file metadata |
| planned | GET | `/api/client/files/{fileId}/download` | user | Download file bytes |
| planned | GET | `/api/client/files/{fileId}/preview` | user | Preview/stream supported file types |
| planned | PATCH | `/api/client/files/{fileId}` | user | Rename file |
| planned | PATCH | `/api/client/files/{fileId}/move` | user | Move file to another folder |
| planned | DELETE | `/api/client/files/{fileId}` | user | Soft-delete file (to trash) |
| planned | POST | `/api/client/files/{fileId}/restore` | user | Restore file from trash |
| planned | DELETE | `/api/client/files/{fileId}/hard` | user | Permanently delete file |
| planned | POST | `/api/client/files/{fileId}/copy` | user | Copy file |

## 7. Trash

| Status | Method | Path | Auth | Purpose |
|---|---|---|---|---|
| planned | GET | `/api/client/trash` | user | List deleted files and folders |
| planned | POST | `/api/client/trash/restore` | user | Restore one or multiple deleted items |
| planned | DELETE | `/api/client/trash/items/{itemType}/{itemId}` | user | Permanently delete one trash item |
| planned | DELETE | `/api/client/trash/empty` | user | Empty trash for current user |

## 8. Permissions and Sharing

| Status | Method | Path | Auth | Purpose |
|---|---|---|---|---|
| planned | GET | `/api/client/folders/{folderId}/permissions` | user | List folder permissions |
| planned | POST | `/api/client/folders/{folderId}/permissions` | user | Grant folder permission (`read` or `edit`) |
| planned | PATCH | `/api/client/folders/{folderId}/permissions/{permissionId}` | user | Update folder permission |
| planned | DELETE | `/api/client/folders/{folderId}/permissions/{permissionId}` | user | Revoke folder permission |
| planned | GET | `/api/client/files/{fileId}/permissions` | user | List file permissions |
| planned | POST | `/api/client/files/{fileId}/permissions` | user | Grant file permission (`read` or `edit`) |
| planned | PATCH | `/api/client/files/{fileId}/permissions/{permissionId}` | user | Update file permission |
| planned | DELETE | `/api/client/files/{fileId}/permissions/{permissionId}` | user | Revoke file permission |
| planned | GET | `/api/client/shares/received` | user | List items shared with me |
| planned | GET | `/api/client/shares/sent` | user | List items I shared to others |

## 9. Admin User Management

| Status | Method | Path | Auth | Purpose |
|---|---|---|---|---|
| planned | GET | `/api/admin/users` | admin | List users with paging/filtering |
| planned | POST | `/api/admin/users` | admin | Create new user/admin |
| planned | GET | `/api/admin/users/{userId}` | admin | Get user details |
| planned | PATCH | `/api/admin/users/{userId}` | admin | Update user profile and status |
| planned | PATCH | `/api/admin/users/{userId}/quota` | admin | Update per-user storage quota |
| planned | PATCH | `/api/admin/users/{userId}/role` | admin | Change user role (`admin` or `user`) |
| planned | DELETE | `/api/admin/users/{userId}` | admin | Deactivate or delete user |

## 10. Admin System Settings

| Status | Method | Path | Auth | Purpose |
|---|---|---|---|---|
| planned | GET | `/api/admin/system/settings` | admin | Read system settings |
| planned | PATCH | `/api/admin/system/settings` | admin | Update editable system settings |
| planned | GET | `/api/admin/system/storage/usage` | admin | System-wide storage usage overview |
| planned | POST | `/api/admin/system/storage/recalculate` | admin | Recalculate storage usage values |
| planned | GET | `/api/admin/roles` | admin | List system roles |

## 11. Audit and Operations

| Status | Method | Path | Auth | Purpose |
|---|---|---|---|---|
| planned | GET | `/api/admin/audit-logs` | admin | List audit logs with filters |
| planned | GET | `/api/admin/audit-logs/{logId}` | admin | Get one audit log entry |
| planned | GET | `/api/admin/stats` | admin | Dashboard counters and summaries |
| planned | GET | `/api/admin/health` | admin | Admin diagnostics endpoint |

## 12. Suggested Implementation Order

1. Complete auth/session: refresh, logout-all, sessions list/revoke.
2. Implement folder/file CRUD with soft-delete.
3. Implement upload/download and storage usage updates.
4. Implement trash APIs.
5. Implement permission/share APIs.
6. Implement admin user and settings APIs.
7. Implement audit filtering and stats endpoints.

## 13. Current Endpoints in Code (Snapshot)

Already routed in `src/http/router.rs`:

- `GET /api/client/status`
- `POST /api/client/auth/login`
- `POST /api/client/auth/logout`
- `GET /api/client/me`
- `GET /api/client/storage/list`
- `GET /api/setup/status`
- `POST /api/setup/initialize`

