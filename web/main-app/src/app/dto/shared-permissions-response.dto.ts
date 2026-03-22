import { SharedPermissionTargetDto } from './shared-permission-target.dto';

export interface SharedPermissionsResponseDto {
  resourceType: 'folder' | 'file';
  resourceId: number;
  resourceName: string;
  entries: SharedPermissionTargetDto[];
}
