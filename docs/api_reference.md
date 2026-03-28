# server-pcloud API Reference

## Overview

`server-pcloud` exposes two HTTP surfaces:

- Client API and web app, bound by `PCLOUD_CLIENT_BIND` and defaulting to `0.0.0.0:8080`
- Admin setup API and web app, bound by `PCLOUD_ADMIN_BIND` and defaulting to `127.0.0.1:9090`

The admin setup surface is available only before the system is initialized. After setup completes, the client surface continues serving and the admin setup surface is disabled.

## Authentication

Most client endpoints require:

```http
Authorization: Bearer <access_token>
```

Access tokens are returned by `POST /api/client/auth/login`. Sessions last 12 hours.

The following endpoints also accept `accessToken` as a query parameter when no `Authorization` header is present:

- `GET /api/client/users/profile-image`
- `GET /api/client/storage/files/download`
- `GET /api/client/storage/files/preview`

## Common Error Format

Errors are JSON responses shaped like:

```json
{
  "error": "Human-readable message"
}
```

Used status codes:

- `400 Bad Request`
- `401 Unauthorized`
- `409 Conflict`
- `500 Internal Server Error`

## Shared Response Shapes

### Auth User

```json
{
  "id": 1,
  "username": "admin",
  "fullName": "System Admin",
  "role": "admin",
  "storageQuotaBytes": 1073741824,
  "storageUsedBytes": 12345,
  "profileImageUrl": "/api/client/me/profile-image"
}
```

### Storage Entry

```json
{
  "id": 10,
  "name": "docs",
  "path": "/docs",
  "entryType": "folder",
  "ownerUserId": 2,
  "ownerUsername": "alice",
  "createdByUserId": 2,
  "createdByUsername": "alice",
  "isStarred": false,
  "sizeBytes": null,
  "modifiedAtUnixMs": 1740000000000
}
```

`entryType` is always `folder` or `file`.

## Setup API

These routes are exposed on the admin setup server before initialization, and `GET /api/setup/status` is also available on the client server.

### GET `/api/setup/status`

Returns whether the system has already been initialized.

Response:

```json
{
  "isInitialized": false
}
```

### POST `/api/setup/initialize`

Initializes the system, creates the first admin account, prepares storage directories, and marks the installation as initialized.

Request:

```json
{
  "admin": {
    "username": "admin",
    "email": "admin@example.com",
    "fullName": "System Admin",
    "password": "StrongPass123",
    "passwordConfirmation": "StrongPass123"
  },
  "system": {
    "storageRootPath": "/absolute/path/to/storage",
    "totalStorageLimitBytes": 1099511627776
  }
}
```

Validation and behavior:

- `storageRootPath` must be an absolute path and cannot be `/`
- `totalStorageLimitBytes` is optional, but if supplied it must be positive
- admin username must be 3-32 chars and may contain letters, digits, `.`, `-`, `_`
- full name must be 1-120 chars
- email must include `@` and a dotted domain
- password must match confirmation
- password must include uppercase, lowercase, and numeric characters
- the code currently checks for length `< 8`, although the returned error message says "at least 12 characters long"
- setup can run only once

Response:

```json
{
  "isInitialized": true,
  "message": "Initial system setup completed successfully"
}
```

## System API

### GET `/api/client/status`

Health-style endpoint for the client server.

Response:

```json
{
  "status": "server running",
  "isInitialized": true
}
```

## Auth API

### POST `/api/client/auth/login`

Authenticates an active user and creates a session.

Request:

```json
{
  "username": "alice",
  "password": "StrongPass123"
}
```

Response:

```json
{
  "accessToken": "plain-text-session-token",
  "tokenType": "Bearer",
  "expiresAt": "2026-03-28T12:34:56Z",
  "user": {
    "id": 2,
    "username": "alice",
    "fullName": "Alice Example",
    "role": "user",
    "storageQuotaBytes": 1073741824,
    "storageUsedBytes": 12345,
    "profileImageUrl": "/api/client/me/profile-image"
  }
}
```

Notes:

- only users with `status = active` can log in
- invalid credentials return `401`

### POST `/api/client/auth/logout`

Revokes the current session.

Auth: bearer token required

Response:

```json
{
  "message": "Signed out successfully"
}
```

### GET `/api/client/me`

Returns the authenticated user.

Auth: bearer token required

Response:

```json
{
  "user": {
    "id": 2,
    "username": "alice",
    "fullName": "Alice Example",
    "role": "user",
    "storageQuotaBytes": 1073741824,
    "storageUsedBytes": 12345,
    "profileImageUrl": "/api/client/me/profile-image"
  }
}
```

### GET `/api/client/me/profile-image`

Returns the authenticated user's profile image bytes.

Auth: bearer token required

Response:

- binary body
- `Content-Type` is the stored image MIME type

### POST `/api/client/me/profile-image`

Uploads or replaces the authenticated user's profile image.

Auth: bearer token required

Request: `multipart/form-data`

- field `image`: required file field

Validation and behavior:

- max size is 30 MB
- supported types: `image/png`, `image/jpeg`, `image/webp`, `image/gif`

Response:

```json
{
  "message": "Profile image updated successfully",
  "user": {
    "id": 2,
    "username": "alice",
    "fullName": "Alice Example",
    "role": "user",
    "storageQuotaBytes": 1073741824,
    "storageUsedBytes": 12345,
    "profileImageUrl": "/api/client/me/profile-image"
  }
}
```

### GET `/api/client/users/profile-image`

Returns another user's profile image.

Auth:

- either bearer token
- or `accessToken` query parameter

Query parameters:

- `userId` required
- `accessToken` optional when `Authorization` is not present

Response:

- binary body
- `Content-Type` is the stored image MIME type

## Admin User API

These routes are served from the client API surface and require an authenticated user with role `admin`.

### GET `/api/client/admin/users`

Lists all users.

Response:

```json
{
  "users": [
    {
      "id": 2,
      "username": "alice",
      "email": "alice@example.com",
      "fullName": "Alice Example",
      "role": "user",
      "status": "active",
      "storageQuotaBytes": 1073741824,
      "storageUsedBytes": 12345,
      "createdAtUnixMs": 1740000000000
    }
  ]
}
```

### POST `/api/client/admin/users`

Creates a regular user.

Request:

```json
{
  "username": "bob",
  "email": "bob@example.com",
  "fullName": "Bob Example",
  "password": "StrongPass123",
  "passwordConfirmation": "StrongPass123",
  "storageQuotaBytes": 1073741824
}
```

Validation and behavior:

- username, email, and full name rules match setup validation
- password must be at least 8 chars and include uppercase, lowercase, and numeric characters
- `storageQuotaBytes` must be zero or positive
- created users are given role `user`

Response:

```json
{
  "message": "User created successfully",
  "user": {
    "id": 3,
    "username": "bob",
    "email": "bob@example.com",
    "fullName": "Bob Example",
    "role": "user",
    "status": "active",
    "storageQuotaBytes": 1073741824,
    "storageUsedBytes": 0,
    "createdAtUnixMs": 1740000000000
  }
}
```

### PUT `/api/client/admin/users/:user_id`

Updates a user.

Request:

```json
{
  "username": "bob",
  "email": "bob@example.com",
  "fullName": "Bob Example",
  "storageQuotaBytes": 2147483648
}
```

Validation and behavior:

- `storageQuotaBytes` cannot be negative
- `storageQuotaBytes` cannot be smaller than the user's current `storageUsedBytes`

Response:

```json
{
  "message": "User updated successfully",
  "user": {
    "id": 3,
    "username": "bob",
    "email": "bob@example.com",
    "fullName": "Bob Example",
    "role": "user",
    "status": "active",
    "storageQuotaBytes": 2147483648,
    "storageUsedBytes": 12345,
    "createdAtUnixMs": 1740000000000
  }
}
```

### DELETE `/api/client/admin/users/:user_id`

Deletes a non-admin user and removes their sessions, storage tree, permissions, and profile image.

Validation and behavior:

- admin users cannot be deleted

Response:

```json
{
  "message": "User deleted successfully"
}
```

## Storage Listing and Search

### GET `/api/client/storage/list`

Lists a folder's contents.

Auth: bearer token required

Query parameters:

- `path` optional, defaults to `/`
- `folderId` optional alternative to `path`
- `q` optional in-folder search text
- `limit` optional, default `200`, max `500`
- `cursor` optional numeric offset

Response:

```json
{
  "currentPath": "/docs",
  "currentFolderId": 20,
  "parentFolderId": 10,
  "parentPath": "/",
  "currentPrivilege": "owner",
  "entries": [],
  "nextCursor": "200",
  "hasMore": false,
  "totalStorageLimitBytes": 1099511627776,
  "totalStorageUsedBytes": 5000000,
  "userStorageQuotaBytes": 1073741824,
  "userStorageUsedBytes": 12345
}
```

Notes:

- `currentPrivilege` is `owner`, `editor`, or `viewer`
- `folderId` can be used for shared folders; path lookup is effectively for the caller's own namespace

### GET `/api/client/storage/trash/list`

Lists the authenticated user's trash view.

Query parameters:

- `q` optional search text

Response notes:

- `currentPath` is always `/trash`
- `currentPrivilege` is always `owner`
- current implementation does not paginate trash results

### GET `/api/client/storage/starred/list`

Lists the authenticated user's starred items.

Query parameters:

- `q` optional search text

Response notes:

- `currentPath` is always `/starred`
- current implementation does not paginate starred results

### GET `/api/client/storage/shared/list`

Lists resources shared with the authenticated user.

Query parameters:

- `q` optional filter by resource name or owner username

Response:

```json
{
  "entries": [
    {
      "resourceType": "folder",
      "resourceId": 99,
      "name": "Designs",
      "path": "/Designs",
      "ownerUserId": 5,
      "ownerUsername": "alice",
      "createdByUserId": 5,
      "createdByUsername": "alice",
      "privilegeType": "viewer",
      "dateSharedUnixMs": 1740000000000
    }
  ]
}
```

Notes:

- nested resources under already shared parent folders are filtered out to reduce duplication

### GET `/api/client/search`

Global resource search across accessible content.

Query parameters:

- `q` optional search text
- `limit` optional, default `120`, max `300`
- `cursor` optional numeric offset

Response:

```json
{
  "query": "report",
  "entries": [
    {
      "resourceType": "file",
      "resourceId": 55,
      "name": "report.pdf",
      "path": "/Work/report.pdf",
      "ownerUserId": 2,
      "ownerUsername": "alice",
      "createdByUserId": 2,
      "createdByUsername": "alice",
      "sourceContext": "owned",
      "privilegeType": "owner",
      "navigateFolderId": 20,
      "sizeBytes": 123456,
      "modifiedAtUnixMs": 1740000000000
    }
  ],
  "nextCursor": null,
  "hasMore": false
}
```

Notes:

- empty or blank `q` returns an empty result set

### GET `/api/client/storage/folders/metadata`

Returns folder metadata.

Query parameters:

- `path` optional, defaults to `/`
- `folderId` optional alternative to `path`

Response:

```json
{
  "name": "docs",
  "path": "/docs",
  "ownerUsername": "alice",
  "currentPrivilege": "editor",
  "createdAtUnixMs": 1740000000000,
  "modifiedAtUnixMs": 1740000000000,
  "folderCount": 3,
  "fileCount": 10,
  "totalItemCount": 13
}
```

### GET `/api/client/storage/files/metadata`

Returns file metadata.

Query parameters:

- `fileId` required

Response:

```json
{
  "id": 55,
  "folderId": 20,
  "folderPath": "/docs",
  "ownerUserId": 2,
  "ownerUsername": "alice",
  "currentPrivilege": "viewer",
  "name": "report.pdf",
  "path": "/docs/report.pdf",
  "sizeBytes": 123456,
  "mimeType": "application/pdf",
  "extension": "pdf",
  "isStarred": false,
  "createdAtUnixMs": 1740000000000,
  "modifiedAtUnixMs": 1740000000000
}
```

## Storage Mutation API

### POST `/api/client/storage/folders`

Creates a folder.

Request:

```json
{
  "parentPath": "/docs",
  "parentFolderId": 20,
  "name": "drafts"
}
```

Behavior:

- use either `parentPath` or `parentFolderId`
- if both are omitted, the folder is created under `/`
- creating inside a shared folder requires `editor` permission
- ownership follows the parent folder owner, not necessarily the acting user

Response:

```json
{
  "message": "Folder created successfully",
  "entry": {
    "id": 21,
    "name": "drafts",
    "path": "/docs/drafts",
    "entryType": "folder",
    "ownerUserId": 2,
    "ownerUsername": "alice",
    "createdByUserId": 7,
    "createdByUsername": "bob",
    "isStarred": false,
    "sizeBytes": null,
    "modifiedAtUnixMs": 1740000000000
  }
}
```

### PUT `/api/client/storage/folders`

Renames a folder.

Request:

```json
{
  "path": "/docs/drafts",
  "resourceId": 21,
  "newName": "archive"
}
```

Behavior:

- use `resourceId` when renaming shared folders
- path-based rename is for the caller's own namespace
- root folder cannot be renamed
- renaming requires `editor` permission

### DELETE `/api/client/storage/folders`

Soft-deletes a folder into trash.

Query parameters:

- `path` required

Response:

```json
{
  "message": "Folder moved to trash",
  "deletedPath": "/docs/archive",
  "entryType": "folder",
  "reclaimedBytes": 0
}
```

Notes:

- deletes the full subtree logically
- root folder cannot be deleted

### POST `/api/client/storage/files/upload`

Uploads a file.

Auth: bearer token required

Request: `multipart/form-data`

- `file`: required file field
- `path`: optional target folder path
- `folderId`: optional target folder id

Behavior:

- if `path` and `folderId` are both omitted, upload goes to `/`
- 5 GB request limit
- uploading into a shared folder requires `editor` permission
- ownership follows the destination folder owner
- upload fails with `409` if user quota or system limit would be exceeded

Response:

```json
{
  "message": "File uploaded successfully",
  "entry": {
    "id": 55,
    "name": "report.pdf",
    "path": "/docs/report.pdf",
    "entryType": "file",
    "ownerUserId": 2,
    "ownerUsername": "alice",
    "createdByUserId": 7,
    "createdByUsername": "bob",
    "isStarred": false,
    "sizeBytes": 123456,
    "modifiedAtUnixMs": 1740000000000
  }
}
```

### PUT `/api/client/storage/files`

Renames a file.

Request:

```json
{
  "path": "/docs/report.pdf",
  "resourceId": 55,
  "newName": "report-final.pdf"
}
```

Behavior:

- use `resourceId` when renaming shared files
- path-based rename is for the caller's own namespace
- renaming requires `editor` permission

### DELETE `/api/client/storage/files`

Soft-deletes a file into trash.

Query parameters:

- `path` required

Response:

```json
{
  "message": "File moved to trash",
  "deletedPath": "/docs/report-final.pdf",
  "entryType": "file",
  "reclaimedBytes": 0
}
```

### PUT `/api/client/storage/starred`

Sets or clears the starred flag on an owned resource.

Request:

```json
{
  "path": "/docs/report-final.pdf",
  "entryType": "file",
  "starred": true
}
```

Behavior:

- `entryType` must be `folder` or `file`
- path must point to a resource in the caller's own namespace
- shared resources are not starred through this endpoint
- root folder cannot be starred

Response:

```json
{
  "message": "Star status updated successfully",
  "entry": {
    "id": 55,
    "name": "report-final.pdf",
    "path": "/docs/report-final.pdf",
    "entryType": "file",
    "ownerUserId": 2,
    "ownerUsername": "alice",
    "createdByUserId": 7,
    "createdByUsername": "bob",
    "isStarred": true,
    "sizeBytes": 123456,
    "modifiedAtUnixMs": 1740000000000
  }
}
```

### POST `/api/client/storage/move`

Moves owned files and folders into another owned folder.

Request:

```json
{
  "destinationFolderId": 20,
  "items": [
    {
      "entryType": "folder",
      "resourceId": 21
    },
    {
      "entryType": "file",
      "resourceId": 55
    }
  ]
}
```

Behavior:

- destination folder must be owned by the caller
- moved items must also be owned by the caller
- max 500 items per request
- duplicate items are deduplicated
- root folder cannot be moved
- a folder cannot be moved into itself or its descendant

Response:

```json
{
  "message": "Items moved successfully",
  "movedCount": 2,
  "destinationFolderId": 20,
  "destinationPath": "/docs"
}
```

## Trash API

### DELETE `/api/client/storage/trash/files`

Permanently deletes a trashed file.

Query parameters:

- `path` required

Response:

```json
{
  "message": "File permanently deleted",
  "deletedPath": "/docs/report-final.pdf",
  "entryType": "file",
  "reclaimedBytes": 123456
}
```

### DELETE `/api/client/storage/trash/folders`

Permanently deletes a trashed folder subtree.

Query parameters:

- `path` required

Response:

```json
{
  "message": "Folder permanently deleted",
  "deletedPath": "/docs/archive",
  "entryType": "folder",
  "reclaimedBytes": 456789
}
```

### POST `/api/client/storage/trash/files/restore`

Restores a trashed file.

Query parameters:

- `path` required

Response:

```json
{
  "message": "File restored from trash",
  "restoredPath": "/docs/report-final.pdf",
  "entryType": "file"
}
```

### POST `/api/client/storage/trash/folders/restore`

Restores a trashed folder subtree.

Query parameters:

- `path` required

Behavior:

- fails if the parent folder is still in trash

Response:

```json
{
  "message": "Folder restored from trash",
  "restoredPath": "/docs/archive",
  "entryType": "folder"
}
```

## Download and Preview API

### GET `/api/client/storage/files/download`

Downloads a file as an attachment.

Auth:

- either bearer token
- or `accessToken` query parameter

Query parameters:

- `path` optional for owned-file lookup
- `fileId` optional for accessible/shared-file lookup
- `accessToken` optional when `Authorization` is not present

Response:

- binary stream
- `Content-Disposition: attachment`
- `Accept-Ranges: bytes`

Notes:

- use `fileId` for shared file downloads
- path-based lookup only resolves files in the caller's own namespace

### GET `/api/client/storage/files/preview`

Same lookup rules as download, but streamed inline.

Response:

- binary stream
- `Content-Disposition: inline`
- `Accept-Ranges: bytes`

### POST `/api/client/storage/downloads/batch`

Builds and downloads a zip archive containing selected files and folders.

Request:

```json
{
  "items": [
    {
      "entryType": "folder",
      "resourceId": 21
    },
    {
      "entryType": "file",
      "resourceId": 55
    }
  ]
}
```

Behavior:

- authentication required
- max 1000 resources per request
- duplicate resources are ignored
- supports accessible shared resources as long as the caller can access them

Response:

- binary zip stream
- `Content-Type: application/zip`
- `Content-Disposition: attachment; filename="pcloud-batch-<timestamp>.zip"`

## Sharing API

### GET `/api/client/storage/shares/users`

Searches active users that can be targeted for sharing.

Query parameters:

- `q` optional search text

Response:

```json
{
  "users": [
    {
      "userId": 7,
      "username": "bob",
      "fullName": "Bob Example"
    }
  ]
}
```

Notes:

- excludes the authenticated user
- result set is capped at 20 users

### GET `/api/client/storage/shares`

Lists sharing permissions for a resource.

Query parameters:

- `entryType` required, `folder` or `file`
- `resourceId` required

Behavior:

- only the resource owner can manage or inspect sharing from this endpoint

Response:

```json
{
  "resourceType": "folder",
  "resourceId": 21,
  "resourceName": "archive",
  "entries": [
    {
      "userId": 7,
      "username": "bob",
      "fullName": "Bob Example",
      "privilegeType": "viewer",
      "createdAtUnixMs": 1740000000000
    }
  ]
}
```

### PUT `/api/client/storage/shares`

Creates or replaces a user's permission on a resource.

Request:

```json
{
  "entryType": "folder",
  "resourceId": 21,
  "targetUserId": 7,
  "privilegeType": "editor"
}
```

Behavior:

- only the owner can grant permissions
- `privilegeType` accepts `viewer`, `view`, `read`, `editor`, or `edit`
- stored result is normalized to `viewer` or `editor`
- cannot share with yourself

Response:

```json
{
  "message": "Sharing permissions updated"
}
```

### DELETE `/api/client/storage/shares`

Removes a user's permission from a resource.

Query parameters:

- `entryType` required, `folder` or `file`
- `resourceId` required
- `targetUserId` required

Behavior:

- only the owner can remove permissions
- owner permissions cannot be removed from self

Response:

```json
{
  "message": "Permission removed"
}
```

## Important Behavior Notes

- Paths are normalized to a leading-slash format such as `/docs/report.pdf`
- Path segments containing `..` or backslashes are rejected
- file and folder names cannot be empty, cannot contain path separators, and are limited to 255 characters
- soft delete moves resources into a logical trash state; permanent delete also removes storage bytes from quota tracking
- shared editors can create, rename, and upload into shared folders when they act by folder or file id
- move, delete, restore, and starring are effectively owner-scope operations
