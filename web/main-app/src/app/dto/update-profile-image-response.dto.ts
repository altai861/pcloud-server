import { AuthUserDto } from './auth-user.dto';

export interface UpdateProfileImageResponseDto {
  message: string;
  user: AuthUserDto;
}
