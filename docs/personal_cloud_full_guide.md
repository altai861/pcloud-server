# Personal Cloud System — Full System Guide for Codex

## Purpose

This document defines the **complete system requirements, architecture, and implementation guidelines** for the Personal Cloud System.

It is intended to guide AI-assisted coding (Codex) to implement the system correctly and consistently.

---

# 1. System Overview

The Personal Cloud System is a self-hosted cloud storage platform where users can:

- store files
- organize them in folders
- share access with other users
- access via web and mobile apps

## Architecture

- Backend server (core logic)
- PostgreSQL database (metadata)
- Physical file storage (disk)
- Web app (admin + user UI)
- iOS app (user client)

---

# 2. Core Design Principles

## 2.1 Separation of Concerns

- Database → metadata
- File system → actual file contents
- Backend → business logic
- Clients → UI only

## 2.2 Source of Truth

- Database is the source of truth for structure and permissions
- File system mirrors logical structure but is not authoritative

## 2.3 Physical Storage Model

- Files stored on disk
- Paths derived as:
  full_path = storage_root_path + storage_path

- storage_path is relative

---

# 3. Authentication System

## Model

- Access Token (short-lived, JWT)
- Refresh Token (long-lived)

## Sessions Table

Used for:
- refresh token tracking
- device sessions
- token revocation

## Rules

- Access token → stateless
- Refresh token → stored as hash
- One session = one device login

---

# 4. User & Role System

## Roles

- admin
- user

## Users

Each user has:
- credentials
- storage quota
- owned files/folders

Admin capabilities:
- create/delete users
- set quotas
- manage system settings

---

# 5. File & Folder System

## Folder Structure

- Recursive hierarchy using parent_folder_id
- Root folders per user

## File Properties

- metadata stored in DB
- actual file stored on disk

## Operations

- create folder
- upload file
- download file
- rename
- move
- delete (soft delete)

---

# 6. Permissions System

## Ownership

- owner_user_id defines owner
- owner has full control

## Permission Types

- read
- edit

## Rules

- owner can grant permissions
- non-owner cannot change ownership
- permissions stored in:
  - folder_permissions
  - file_permissions

---

# 7. Deletion System (Trash)

## Soft Delete

- is_deleted = true
- deleted_at timestamp

## Restore

- restore sets is_deleted = false

## Permanent Delete

- removes DB row
- deletes file from disk

---

# 8. Storage Management

## Root Path

- defined during setup
- stored in system_settings
- used to resolve all file paths

## Storage Limit

- total system limit
- per-user quota

## Important Rule

Changing storage_root_path requires migration.

---

# 9. System Settings

Table: system_settings

Stores:
- is_initialized
- storage_root_path
- total_storage_limit_bytes
- default_user_quota_bytes

---

# 10. Audit Logging

Tracks:
- user actions
- file operations
- permission changes

---

# 11. API Design (High Level)

## Setup

- GET /api/setup/status
- POST /api/setup/initialize

## Auth

- POST /api/auth/login
- POST /api/auth/refresh
- POST /api/auth/logout

## Users

- GET /api/users
- POST /api/users
- DELETE /api/users/{id}

## Files

- POST /api/files/upload
- GET /api/files/{id}
- DELETE /api/files/{id}

## Folders

- POST /api/folders
- GET /api/folders/{id}
- DELETE /api/folders/{id}

## Permissions

- POST /api/permissions

---

# 12. Backend Architecture

Recommended layers:

- Controller (API layer)
- Service (business logic)
- Repository (DB access)
- FileService (disk operations)
- AuthService (tokens)

---

# 13. Important Constraints

- usernames unique
- emails unique
- permission uniqueness
- privilege_type in ('read', 'edit')

---

# 14. Error Handling

- validate all inputs
- return clear errors
- avoid leaking sensitive info

---

# 15. Security Requirements

- hash passwords
- hash refresh tokens
- validate permissions on every request
- restrict file access by ownership/permissions

---

# 16. Implementation Order

## Phase 1
- setup + admin creation

## Phase 2
- auth system

## Phase 3
- user management

## Phase 4
- file & folder system

## Phase 5
- permissions

## Phase 6
- audit + polish

---

# 17. What Codex Should NOT Do

- change database schema arbitrarily
- remove soft delete
- ignore permissions model
- store files in DB

---

# 18. Final Goal

A working system that supports:

- admin setup
- user login
- file storage
- folder hierarchy
- permissions
- multi-device sessions

