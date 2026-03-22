export interface AdminCreateUserRequestDto {
  username: string;
  email: string;
  fullName: string;
  password: string;
  passwordConfirmation: string;
  storageQuotaBytes: number;
}
