export interface StorageEntryDto {
  name: string;
  path: string;
  entryType: 'folder' | 'file';
  isStarred: boolean;
  sizeBytes: number | null;
  modifiedAtUnixMs: number | null;
}
