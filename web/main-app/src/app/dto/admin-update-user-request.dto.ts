export interface AdminUpdateUserRequestDto {
  username: string;
  email: string;
  fullName: string;
  storageQuotaBytes: number;
}
