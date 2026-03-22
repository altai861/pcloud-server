export interface SharedPermissionTargetDto {
  userId: number;
  username: string;
  fullName: string;
  privilegeType: 'viewer' | 'editor';
  createdAtUnixMs: number;
}
