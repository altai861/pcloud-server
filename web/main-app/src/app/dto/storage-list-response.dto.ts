import { StorageEntryDto } from './storage-entry.dto';

export interface StorageListResponseDto {
  currentPath: string;
  parentPath: string | null;
  entries: StorageEntryDto[];
  totalStorageLimitBytes: number | null;
  totalStorageUsedBytes: number;
  userStorageQuotaBytes: number;
  userStorageUsedBytes: number;
}
