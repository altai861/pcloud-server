export interface StorageFolderMetadataDto {
  name: string;
  path: string;
  ownerUsername: string;
  currentPrivilege: 'owner' | 'editor' | 'viewer';
  createdAtUnixMs: number;
  modifiedAtUnixMs: number;
  folderCount: number;
  fileCount: number;
  totalItemCount: number;
}
