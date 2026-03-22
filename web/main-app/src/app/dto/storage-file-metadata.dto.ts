export interface StorageFileMetadataDto {
  id: number;
  folderId: number;
  folderPath: string;
  ownerUserId: number;
  ownerUsername: string;
  currentPrivilege: 'owner' | 'editor' | 'viewer';
  name: string;
  path: string;
  sizeBytes: number;
  mimeType: string | null;
  extension: string | null;
  isStarred: boolean;
  createdAtUnixMs: number;
  modifiedAtUnixMs: number;
}
