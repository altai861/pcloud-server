export interface StorageRestoreResponseDto {
  message: string;
  restoredPath: string;
  entryType: 'folder' | 'file';
}
