import { StorageEntryDto } from './storage-entry.dto';

export interface StorageListResponseDto {
  currentPath: string;
  currentFolderId: number | null;
  parentFolderId: number | null;
  parentPath: string | null;
  currentPrivilege: 'owner' | 'editor' | 'viewer';
  entries: StorageEntryDto[];
  totalStorageLimitBytes: number | null;
  totalStorageUsedBytes: number;
  userStorageQuotaBytes: number;
  userStorageUsedBytes: number;
}
