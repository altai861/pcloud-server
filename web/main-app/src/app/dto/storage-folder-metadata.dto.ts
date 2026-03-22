export interface StorageFolderMetadataDto {
  name: string;
  path: string;
  createdAtUnixMs: number;
  modifiedAtUnixMs: number;
  folderCount: number;
  fileCount: number;
  totalItemCount: number;
}
