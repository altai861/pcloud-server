import { AdminUserDto } from './admin-user.dto';

export interface AdminCreateUserResponseDto {
  message: string;
  user: AdminUserDto;
}
