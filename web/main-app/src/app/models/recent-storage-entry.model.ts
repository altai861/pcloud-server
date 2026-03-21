import { StorageEntryDto } from '../dto/storage-entry.dto';

export interface RecentStorageEntryModel extends StorageEntryDto {
  openedAtUnixMs: number;
}
