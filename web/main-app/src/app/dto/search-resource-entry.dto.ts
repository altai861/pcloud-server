export interface SearchResourceEntryDto {
  resourceType: 'folder' | 'file';
  resourceId: number;
  name: string;
  path: string;
  ownerUserId: number;
  ownerUsername: string;
  createdByUserId: number | null;
  createdByUsername: string;
  sourceContext: 'storage' | 'shared';
  privilegeType: 'owner' | 'editor' | 'viewer';
  navigateFolderId: number;
  sizeBytes: number | null;
  modifiedAtUnixMs: number;
}
