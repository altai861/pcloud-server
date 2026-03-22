export interface SharedResourceEntryDto {
  resourceType: 'folder' | 'file';
  resourceId: number;
  name: string;
  path: string;
  ownerUserId: number;
  ownerUsername: string;
  createdByUserId: number | null;
  createdByUsername: string;
  privilegeType: 'viewer' | 'editor';
  dateSharedUnixMs: number;
}
