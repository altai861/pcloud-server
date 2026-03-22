export interface StorageDeleteResponseDto {
  message: string;
  deletedPath: string;
  entryType: 'folder' | 'file';
  reclaimedBytes: number;
}
