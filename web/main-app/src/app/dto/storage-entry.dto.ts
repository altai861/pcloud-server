export interface StorageEntryDto {
  id: number;
  name: string;
  path: string;
  entryType: 'folder' | 'file';
  ownerUserId: number;
  ownerUsername: string;
  createdByUserId: number | null;
  createdByUsername: string;
  isStarred: boolean;
  sizeBytes: number | null;
  modifiedAtUnixMs: number | null;
}
