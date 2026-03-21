export interface AuthUserDto {
  id: number;
  username: string;
  fullName: string;
  role: string;
  storageQuotaBytes: number;
  storageUsedBytes: number;
}
