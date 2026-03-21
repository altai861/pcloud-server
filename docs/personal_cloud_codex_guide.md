# Personal Cloud System — Coding Guide for Codex

## Purpose

This document describes the current requirements, scope, and implementation guidance for the **Personal Cloud System**. It is intended to be used as an instruction file for coding assistance, especially for implementing the **admin launching / initial setup part** first.

The system is a **personal cloud storage platform** with:
- a backend server
- a web application
- an iOS application
- an admin-managed multi-user system
- physical file storage on the host machine
- PostgreSQL for metadata persistence

The current coding priority is the **initial launch and admin setup flow**.

---

## System Overview

The system allows a person to run their own cloud storage server on their own machine. The server stores:
- **metadata in PostgreSQL**
- **actual file contents in the physical file system**

The physical file system is intended to **mirror the logical folder hierarchy** visible to the user.

The system has two main actor types:
- **Admin**
- **User**

The admin performs the initial system setup and manages users and system-wide settings.

---

## Current Core Design Decisions

### Storage model
- PostgreSQL is used for system metadata.
- Actual file bytes are stored on disk.
- The physical file system should mirror the logical file/folder hierarchy.
- Database remains the source of truth for metadata.

### Permissions model
- Every file and folder has an **owner**.
- Non-owners may receive explicit permissions.
- Supported privilege levels:
  - `read`
  - `edit`
- Only the **owner** may transfer ownership.
- `read` means view/download only.
- `edit` means modify content/metadata.
- Permission tables exist separately for files and folders.

### Deletion model
- The system supports **trash / restore**.
- Files and folders use soft-delete fields:
  - `is_deleted`
  - `deleted_at`

### Authentication model
- Authentication uses **access token + refresh token**.
- Access token is short-lived and can be stateless.
- Refresh token lifecycle is tracked in the database through a `sessions` table.
- `sessions` stores a **refresh_token_hash**, not the raw refresh token.

### System configuration model
- Some values are system-wide configuration values.
- Deployment secrets belong in env/config.
- App-level global settings may be stored in DB.
- `storage_root_path` should be treated as a sensitive system-level setting.
- Changing `storage_root_path` later requires a controlled migration process.

---

## Current PostgreSQL-Oriented Data Model

### `roles`
- `id`
- `name`
- `description`

### `users`
- `id`
- `role_id`
- `username`
- `email`
- `password_hash`
- `full_name`
- `status`
- `storage_quota_bytes`
- `storage_used_bytes`
- `created_at`
- `updated_at`

### `folders`
- `id`
- `owner_user_id`
- `parent_folder_id`
- `name`
- `path`
- `is_deleted`
- `created_at`
- `updated_at`
- `deleted_at`

### `files`
- `id`
- `owner_user_id`
- `folder_id`
- `name`
- `original_file_name`
- `mime_type`
- `extension`
- `size_bytes`
- `storage_path`
- `checksum`
- `is_deleted`
- `created_at`
- `updated_at`
- `deleted_at`

### `folder_permissions`
- `id`
- `folder_id`
- `user_id`
- `privilege_type`
- `granted_by_user_id`
- `created_at`

### `file_permissions`
- `id`
- `file_id`
- `user_id`
- `privilege_type`
- `granted_by_user_id`
- `created_at`

### `sessions`
- `id`
- `user_id`
- `refresh_token_hash`
- `device_info`
- `ip_address`
- `user_agent`
- `expires_at`
- `created_at`
- `updated_at`
- `revoked_at`

### `audit_logs`
- `id`
- `user_id`
- `action_type`
- `target_type`
- `target_id`
- `description`
- `created_at`

### Important constraints
- `users.username` must be unique
- `users.email` must be unique
- `folder_permissions (folder_id, user_id)` must be unique
- `file_permissions (file_id, user_id)` must be unique
- `privilege_type` must be one of:
  - `read`
  - `edit`

---

## Current Coding Priority

## Phase 1: Admin Launching / Initial Setup

The first implementation target is the **very first launch experience** of the system.

This phase should answer these questions:
- Has the system already been initialized?
- If not, how does the first admin configure the system?
- How is the first admin account created?
- How is the storage root directory chosen and validated?
- How are the database and initial records prepared?

This phase is more important than regular user features right now.

---

## Required Features for the Initial Setup Phase

### 1. System initialization check
The backend must be able to determine whether the system has already been initialized.

Suggested logic:
- If no admin user exists, or no initialization marker exists, the system is considered uninitialized.
- Once setup is completed successfully, the system becomes initialized.

Possible implementation options:
- a dedicated `system_settings` or `system_state` table
- or a robust initialization check using existing data

Recommended approach:
- create a dedicated **system settings / state** table and store an initialization flag

Example fields:
- `id`
- `is_initialized`
- `storage_root_path`
- `total_storage_limit_bytes`
- `created_at`
- `updated_at`

---

### 2. First admin creation
On first launch, the system must allow creation of the first admin account.

Required fields:
- username
- email
- full name
- password
- password confirmation

Requirements:
- password must be hashed securely
- role must be admin
- username and email must be unique
- setup must fail safely if creation does not fully complete

---

### 3. Storage root path configuration
During initial setup, the admin chooses the main storage directory.

Requirements:
- path must be validated
- path must be absolute
- path must be creatable if it does not exist
- backend must check read/write permissions
- backend must avoid unsafe paths if needed
- backend must persist the chosen path in system settings
- backend should create the root directory structure if necessary

Suggested initial physical structure:
- `{storage_root_path}/`

Later each user may have a dedicated root under it, for example:
- `{storage_root_path}/{user_id}/`

Because physical hierarchy mirrors logical hierarchy, this root path is important.

---

### 4. Total storage limit configuration
During initial setup, the admin should be able to define the total system storage limit.

Requirements:
- value stored in bytes
- must be a positive integer
- may optionally allow `NULL` or special handling for unlimited storage
- should be persisted in system settings

---

### 5. Setup endpoint / setup flow
The backend should expose a dedicated setup flow for first launch.

Suggested endpoints:
- `GET /api/setup/status`
- `POST /api/setup/initialize`

#### `GET /api/setup/status`
Returns whether setup is already complete.

Example response:
```json
{
  "isInitialized": false
}
```

#### `POST /api/setup/initialize`
Accepts initial system configuration and first admin account data.

Example request:
```json
{
  "admin": {
    "username": "admin",
    "email": "admin@example.com",
    "fullName": "System Administrator",
    "password": "strong-password",
    "passwordConfirmation": "strong-password"
  },
  "system": {
    "storageRootPath": "/srv/pcloud-storage",
    "totalStorageLimitBytes": 100000000000
  }
}
```

Behavior:
- validate input
- verify system is not already initialized
- create required system state/settings record
- create or ensure admin role exists
- create first admin user
- create storage root directory if needed
- mark system initialized
- write audit log if appropriate
- return success result

This process should be **transactional where possible**.

---

## Non-Functional Requirements for Setup

### Safety
- Do not allow setup to run again once initialized.
- Fail safely if any critical step fails.
- Avoid partial system initialization.

### Validation
- Validate all user inputs.
- Validate storage path thoroughly.
- Ensure password confirmation matches.
- Ensure password meets basic strength policy.

### Security
- Hash passwords using a secure password hashing algorithm.
- Never store raw passwords.
- Do not expose sensitive internals in setup errors.
- Protect setup endpoint after initialization.

### Idempotency / consistency
- Setup should either complete successfully or fail cleanly.
- Avoid duplicate admin creation.
- Avoid duplicate role creation.

---

## Suggested `system_settings` / `system_state` Table

Codex may implement one of these names:
- `system_settings`
- `system_state`

Recommended schema:

```sql
id BIGSERIAL PRIMARY KEY
is_initialized BOOLEAN NOT NULL DEFAULT FALSE
storage_root_path TEXT NOT NULL
total_storage_limit_bytes BIGINT
created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
```

Optional future fields:
- default_user_quota_bytes
- allow_registration
- app_display_name

If only one global row is expected, the application may enforce a single-row design.

---

## Expected Setup Flow

1. backend starts
2. backend checks whether system is initialized
3. if not initialized:
   - allow setup endpoints
   - frontend shows setup screen
4. admin submits setup form
5. backend validates all data
6. backend creates required roles if missing
7. backend creates admin user
8. backend creates/validates storage root directory
9. backend stores system settings
10. backend marks system initialized
11. backend returns success
12. frontend redirects to login

---

## Important Edge Cases

Codex should handle these carefully:

### Setup already completed
- `POST /api/setup/initialize` must reject further setup attempts

### Storage path invalid
- path not absolute
- path not writable
- path cannot be created
- path points to invalid or forbidden location

### Duplicate username/email
- must return validation error

### Password mismatch
- must return validation error

### Partial failure
- if DB write succeeds but file system preparation fails, rollback or fail safely

### Missing role
- system should create required default roles if missing

---

## Backend Requirements for Codex

Codex should implement the setup part with the following principles:
- clean module structure
- clear separation of:
  - routing/controller
  - service/business logic
  - database access/repository
  - file system utility logic
  - validation
- use transactions for DB operations where possible
- use strongly typed request/response models if language/framework supports them
- implement proper error handling
- avoid hardcoding paths or secrets
- read deployment-specific values from env/config
- keep system-level app settings in DB

---

## What Should Be Built First

### Minimum first milestone
Codex should first implement these items:
1. PostgreSQL schema/migration for:
   - roles
   - users
   - system_settings or system_state
2. setup status endpoint
3. setup initialize endpoint
4. password hashing
5. storage root path validation and directory creation
6. first admin creation

After that, regular login can be implemented.

---

## Features That Can Wait Until Later

The following do not need to be implemented in the initial setup phase:
- user file upload/download
- folder browsing
- file sharing
- folder/file permissions management UI
- trash/restore logic
- audit log UI
- iOS-specific flows
- advanced storage migration
- ownership transfer logic

They should be preserved in the design, but not block the first launch flow.

---

## Coding Expectations for Codex

Codex should:
- prioritize correctness over cleverness
- generate production-leaning code, not toy examples
- include input validation
- include error handling
- include comments only where helpful
- avoid unnecessary abstraction
- keep naming consistent with this document
- preserve current architecture and DB assumptions

Codex should not:
- redesign the whole schema without reason
- remove the sessions table
- replace PostgreSQL with another database
- replace soft delete design
- introduce unrelated features in this phase

---

## If Codex Needs to Make Reasonable Assumptions

Codex may assume:
- there are two default roles: `admin` and `user`
- setup is a one-time operation
- access token is short-lived
- refresh token sessions will be stored later in the `sessions` table
- physical storage root should be created during setup if missing

But Codex should avoid making assumptions that conflict with this document.

---

## Final Implementation Goal for This Phase

At the end of this phase, the system should support:
- detecting whether the system is initialized
- configuring storage root path
- configuring total storage limit
- creating the first admin account
- persisting initialization state
- preparing the backend for later login and user management features
