export interface StorageEntryDto {
  name: string;
  path: string;
  entryType: 'folder' | 'file';
  sizeBytes: number | null;
  modifiedAtUnixMs: number | null;
}
