import { AuthUserDto } from './auth-user.dto';

export interface LoginResponseDto {
  accessToken: string;
  tokenType: string;
  expiresAt: string;
  user: AuthUserDto;
}
