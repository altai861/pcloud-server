export interface AdminUserDto {
  id: number;
  username: string;
  email: string;
  fullName: string;
  role: string;
  status: string;
  storageQuotaBytes: number;
  storageUsedBytes: number;
  createdAtUnixMs: number;
}
