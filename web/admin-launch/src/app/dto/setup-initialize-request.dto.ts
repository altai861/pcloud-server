import { AdminSetupRequestDto } from './admin-setup-request.dto';
import { SystemSetupRequestDto } from './system-setup-request.dto';

export interface SetupInitializeRequestDto {
  admin: AdminSetupRequestDto;
  system: SystemSetupRequestDto;
}
