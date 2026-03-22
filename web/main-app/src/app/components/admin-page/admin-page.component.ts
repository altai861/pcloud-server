import { CommonModule } from '@angular/common';
import { HttpErrorResponse } from '@angular/common/http';
import { ChangeDetectorRef, Component, HostListener, OnInit } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { Router } from '@angular/router';

import { AdminCreateUserRequestDto } from '../../dto/admin-create-user-request.dto';
import { AdminUpdateUserRequestDto } from '../../dto/admin-update-user-request.dto';
import { AdminUserDto } from '../../dto/admin-user.dto';
import { ApiErrorResponseDto } from '../../dto/api-error-response.dto';
import { AdminApiService } from '../../services/admin-api.service';
import { ClientSessionService } from '../../services/client-session.service';

@Component({
  selector: 'app-admin-page',
  imports: [CommonModule, FormsModule],
  templateUrl: './admin-page.component.html',
  styleUrl: './admin-page.component.css'
})
export class AdminPageComponent implements OnInit {
  loading = true;
  creating = false;
  updating = false;
  deletingUserId: number | null = null;
  openUserActionsMenuId: number | null = null;
  userActionsMenuX = 0;
  userActionsMenuY = 0;
  showCreateUserForm = false;
  showUpdateUserForm = false;
  errorMessage = '';
  actionErrorMessage = '';
  actionSuccessMessage = '';
  createErrorMessage = '';
  updateErrorMessage = '';
  createWarnings: string[] = [];
  updateWarnings: string[] = [];
  users: AdminUserDto[] = [];

  createForm = {
    username: '',
    email: '',
    fullName: '',
    password: '',
    passwordConfirmation: '',
    storageQuotaInput: '',
    storageQuotaBytes: ''
  };

  updateForm = {
    id: null as number | null,
    username: '',
    email: '',
    fullName: '',
    storageQuotaInput: '',
    storageQuotaBytes: ''
  };

  constructor(
    private readonly adminApiService: AdminApiService,
    private readonly sessionService: ClientSessionService,
    private readonly router: Router,
    private readonly cdr: ChangeDetectorRef
  ) {}

  ngOnInit(): void {
    this.loadUsers();
  }

  @HostListener('document:keydown.escape')
  onEscapePressed(): void {
    if (this.openUserActionsMenuId !== null) {
      this.closeUserActionsMenu();
      return;
    }

    if (this.showCreateUserForm && !this.creating) {
      this.closeCreateUserForm();
      return;
    }

    if (this.showUpdateUserForm && !this.updating) {
      this.closeUpdateUserForm();
    }
  }

  @HostListener('document:click')
  onDocumentClick(): void {
    if (this.openUserActionsMenuId === null) {
      return;
    }

    this.closeUserActionsMenu();
  }

  openCreateUserForm(): void {
    this.showCreateUserForm = true;
    this.showUpdateUserForm = false;
    this.closeUserActionsMenu();
    this.actionErrorMessage = '';
    this.actionSuccessMessage = '';
    this.createErrorMessage = '';
    this.createWarnings = [];
  }

  closeCreateUserForm(): void {
    this.showCreateUserForm = false;
    this.createErrorMessage = '';
    this.createWarnings = [];
    this.createForm = {
      username: '',
      email: '',
      fullName: '',
      password: '',
      passwordConfirmation: '',
      storageQuotaInput: '',
      storageQuotaBytes: ''
    };
  }

  openUpdateUserForm(user: AdminUserDto): void {
    this.showUpdateUserForm = true;
    this.showCreateUserForm = false;
    this.closeUserActionsMenu();
    this.actionErrorMessage = '';
    this.actionSuccessMessage = '';
    this.updateErrorMessage = '';
    this.updateWarnings = [];
    this.updateForm = {
      id: user.id,
      username: user.username,
      email: user.email,
      fullName: user.fullName,
      storageQuotaInput: this.formatBytes(user.storageQuotaBytes),
      storageQuotaBytes: String(user.storageQuotaBytes)
    };
  }

  closeUpdateUserForm(): void {
    this.showUpdateUserForm = false;
    this.updateErrorMessage = '';
    this.updateWarnings = [];
    this.updateForm = {
      id: null,
      username: '',
      email: '',
      fullName: '',
      storageQuotaInput: '',
      storageQuotaBytes: ''
    };
  }

  submitCreateUser(): void {
    this.createWarnings = [];
    this.createErrorMessage = '';

    const payload = this.buildCreatePayload();
    if (!payload) {
      return;
    }

    const accessToken = this.sessionService.readAccessToken();
    if (!accessToken) {
      this.sessionService.clearSession();
      this.router.navigate(['/login']);
      return;
    }

    this.creating = true;
    this.actionSuccessMessage = '';

    this.adminApiService.createUser('', accessToken, payload).subscribe({
      next: (response) => {
        this.users = [response.user, ...this.users];
        this.actionSuccessMessage = response.message;
        this.creating = false;
        this.closeCreateUserForm();
        this.cdr.detectChanges();
      },
      error: (error: unknown) => {
        this.creating = false;
        this.createErrorMessage = this.extractError(error, 'Failed to create user');

        if (error instanceof HttpErrorResponse && error.status === 401) {
          this.sessionService.clearSession();
          this.router.navigate(['/login']);
          return;
        }

        this.cdr.detectChanges();
      }
    });
  }

  submitUpdateUser(): void {
    this.updateWarnings = [];
    this.updateErrorMessage = '';

    const payload = this.buildUpdatePayload();
    if (!payload || this.updateForm.id === null) {
      return;
    }

    const accessToken = this.sessionService.readAccessToken();
    if (!accessToken) {
      this.sessionService.clearSession();
      this.router.navigate(['/login']);
      return;
    }

    this.updating = true;
    this.actionSuccessMessage = '';

    this.adminApiService.updateUser('', accessToken, this.updateForm.id, payload).subscribe({
      next: (response) => {
        this.users = this.users.map((user) => (user.id === response.user.id ? response.user : user));
        this.actionSuccessMessage = response.message;
        this.updating = false;
        this.closeUpdateUserForm();
        this.cdr.detectChanges();
      },
      error: (error: unknown) => {
        this.updating = false;
        this.updateErrorMessage = this.extractError(error, 'Failed to update user');

        if (error instanceof HttpErrorResponse && error.status === 401) {
          this.sessionService.clearSession();
          this.router.navigate(['/login']);
          return;
        }

        this.cdr.detectChanges();
      }
    });
  }

  confirmDeleteUser(user: AdminUserDto): void {
    if (this.isDeleteDisabled(user)) {
      this.actionErrorMessage = 'Admin users cannot be deleted.';
      return;
    }

    const shouldDelete = window.confirm(
      `Delete user "${user.username}"? This will permanently remove all owned files and folders.`
    );
    if (!shouldDelete) {
      return;
    }

    const accessToken = this.sessionService.readAccessToken();
    if (!accessToken) {
      this.sessionService.clearSession();
      this.router.navigate(['/login']);
      return;
    }

    this.deletingUserId = user.id;
    this.closeUserActionsMenu();
    this.actionErrorMessage = '';
    this.actionSuccessMessage = '';

    this.adminApiService.deleteUser('', accessToken, user.id).subscribe({
      next: (response) => {
        this.users = this.users.filter((existing) => existing.id !== user.id);
        this.actionSuccessMessage = response.message;
        this.deletingUserId = null;

        if (this.updateForm.id === user.id) {
          this.closeUpdateUserForm();
        }

        this.cdr.detectChanges();
      },
      error: (error: unknown) => {
        this.deletingUserId = null;
        this.actionErrorMessage = this.extractError(error, 'Failed to delete user');

        if (error instanceof HttpErrorResponse && error.status === 401) {
          this.sessionService.clearSession();
          this.router.navigate(['/login']);
          return;
        }

        this.cdr.detectChanges();
      }
    });
  }

  isDeleteDisabled(user: AdminUserDto): boolean {
    return user.role.toLowerCase() === 'admin';
  }

  get openedUserForActionsMenu(): AdminUserDto | null {
    if (this.openUserActionsMenuId === null) {
      return null;
    }

    return this.users.find((user) => user.id === this.openUserActionsMenuId) ?? null;
  }

  toggleUserActionsMenu(event: MouseEvent, userId: number): void {
    event.preventDefault();
    event.stopPropagation();

    if (this.openUserActionsMenuId === userId) {
      this.closeUserActionsMenu();
      return;
    }

    const menuWidth = 170;
    const menuHeight = 92;
    const viewportPadding = 8;
    const triggerRect = (event.currentTarget as HTMLElement | null)?.getBoundingClientRect();

    const preferredLeft = triggerRect
      ? triggerRect.right - menuWidth
      : event.clientX;
    const preferredTop = triggerRect
      ? triggerRect.bottom + 6
      : event.clientY;

    this.userActionsMenuX = Math.max(
      viewportPadding,
      Math.min(preferredLeft, window.innerWidth - menuWidth - viewportPadding)
    );
    this.userActionsMenuY = Math.max(
      viewportPadding,
      Math.min(preferredTop, window.innerHeight - menuHeight - viewportPadding)
    );
    this.openUserActionsMenuId = userId;
  }

  closeUserActionsMenu(): void {
    this.openUserActionsMenuId = null;
  }

  onUpdateFromMenu(event: MouseEvent, user: AdminUserDto): void {
    event.preventDefault();
    event.stopPropagation();
    this.closeUserActionsMenu();
    this.openUpdateUserForm(user);
  }

  onDeleteFromMenu(event: MouseEvent, user: AdminUserDto): void {
    event.preventDefault();
    event.stopPropagation();
    this.closeUserActionsMenu();
    this.confirmDeleteUser(user);
  }

  onFriendlyQuotaChange(): void {
    this.syncFriendlyToBytes(this.createForm);
  }

  onBytesQuotaChange(): void {
    this.syncBytesToFriendly(this.createForm);
  }

  onUpdateFriendlyQuotaChange(): void {
    this.syncFriendlyToBytes(this.updateForm);
  }

  onUpdateBytesQuotaChange(): void {
    this.syncBytesToFriendly(this.updateForm);
  }

  get quotaBytesPreviewText(): string {
    const resolved = this.resolveQuotaBytesFromFields(
      this.createForm.storageQuotaInput,
      this.createForm.storageQuotaBytes
    );
    if (resolved.value === null) {
      return 'Enter size like 4GB and/or exact bytes';
    }

    return `${resolved.value.toLocaleString()} bytes (${this.formatSize(resolved.value)})`;
  }

  get updateQuotaBytesPreviewText(): string {
    const resolved = this.resolveQuotaBytesFromFields(
      this.updateForm.storageQuotaInput,
      this.updateForm.storageQuotaBytes
    );
    if (resolved.value === null) {
      return 'Enter size like 4GB and/or exact bytes';
    }

    return `${resolved.value.toLocaleString()} bytes (${this.formatSize(resolved.value)})`;
  }

  formatSize(bytes: number): string {
    if (!Number.isFinite(bytes) || bytes < 0) {
      return '-';
    }

    const units = ['B', 'KB', 'MB', 'GB', 'TB', 'PB'];
    let value = bytes;
    let index = 0;

    while (value >= 1000 && index < units.length - 1) {
      value /= 1000;
      index += 1;
    }

    if (index === 0) {
      return `${Math.round(value)} ${units[index]}`;
    }

    const rounded = value >= 100 ? value.toFixed(0) : value >= 10 ? value.toFixed(1) : value.toFixed(2);
    const compact = rounded.replace(/\.0+$/, '').replace(/(\.\d*[1-9])0+$/, '$1');

    return `${compact} ${units[index]}`;
  }

  formatDate(unixMillis: number): string {
    if (!Number.isFinite(unixMillis)) {
      return '-';
    }

    const date = new Date(unixMillis);
    if (Number.isNaN(date.getTime())) {
      return '-';
    }

    return new Intl.DateTimeFormat(undefined, {
      year: 'numeric',
      month: 'short',
      day: 'numeric'
    }).format(date);
  }

  private loadUsers(): void {
    const accessToken = this.sessionService.readAccessToken();
    if (!accessToken) {
      this.sessionService.clearSession();
      this.router.navigate(['/login']);
      return;
    }

    this.loading = true;
    this.errorMessage = '';

    this.adminApiService.listUsers('', accessToken).subscribe({
      next: (response) => {
        this.users = response.users;
        this.loading = false;
        this.cdr.detectChanges();
      },
      error: (error: unknown) => {
        this.loading = false;
        this.errorMessage = this.extractError(error, 'Failed to load users');

        if (error instanceof HttpErrorResponse && error.status === 401) {
          this.sessionService.clearSession();
          this.router.navigate(['/login']);
          return;
        }

        this.cdr.detectChanges();
      }
    });
  }

  private buildCreatePayload(): AdminCreateUserRequestDto | null {
    const username = this.createForm.username.trim();
    const email = this.createForm.email.trim();
    const fullName = this.createForm.fullName.trim();
    const password = this.createForm.password;
    const passwordConfirmation = this.createForm.passwordConfirmation;
    const warnings: string[] = [];

    if (!username || !email || !fullName || !password || !passwordConfirmation) {
      warnings.push('Please fill username, email, full name, password and confirmation.');
    }

    if (username && (username.length < 3 || username.length > 32)) {
      warnings.push('Username length must be between 3 and 32 characters.');
    }

    if (username && !/^[A-Za-z0-9._-]+$/.test(username)) {
      warnings.push("Username can contain only letters, numbers, '.', '-' and '_'.");
    }

    if (email && !this.looksLikeEmail(email)) {
      warnings.push('Email format is invalid.');
    }

    if (fullName && fullName.length > 120) {
      warnings.push('Full name must be 120 characters or less.');
    }

    if (password && password.length < 8) {
      warnings.push('Password must be at least 8 characters long.');
    }

    if (password && !/[A-Z]/.test(password)) {
      warnings.push('Password must include at least one uppercase letter.');
    }

    if (password && !/[a-z]/.test(password)) {
      warnings.push('Password must include at least one lowercase letter.');
    }

    if (password && !/[0-9]/.test(password)) {
      warnings.push('Password must include at least one number.');
    }

    if (password !== passwordConfirmation) {
      warnings.push('Password confirmation does not match.');
    }

    const quota = this.resolveQuotaBytesFromFields(
      this.createForm.storageQuotaInput,
      this.createForm.storageQuotaBytes
    );
    if (quota.error) {
      warnings.push(quota.error);
    }

    if (warnings.length > 0 || quota.value === null) {
      this.createWarnings = warnings;
      this.createErrorMessage = warnings[0] ?? 'Please check the form fields.';
      return null;
    }

    return {
      username,
      email,
      fullName,
      password,
      passwordConfirmation,
      storageQuotaBytes: quota.value
    };
  }

  private buildUpdatePayload(): AdminUpdateUserRequestDto | null {
    const username = this.updateForm.username.trim();
    const email = this.updateForm.email.trim();
    const fullName = this.updateForm.fullName.trim();
    const warnings: string[] = [];

    if (!username || !email || !fullName) {
      warnings.push('Please fill username, email, and full name.');
    }

    if (username && (username.length < 3 || username.length > 32)) {
      warnings.push('Username length must be between 3 and 32 characters.');
    }

    if (username && !/^[A-Za-z0-9._-]+$/.test(username)) {
      warnings.push("Username can contain only letters, numbers, '.', '-' and '_'.");
    }

    if (email && !this.looksLikeEmail(email)) {
      warnings.push('Email format is invalid.');
    }

    if (fullName && fullName.length > 120) {
      warnings.push('Full name must be 120 characters or less.');
    }

    const quota = this.resolveQuotaBytesFromFields(
      this.updateForm.storageQuotaInput,
      this.updateForm.storageQuotaBytes
    );
    if (quota.error) {
      warnings.push(quota.error);
    }

    if (warnings.length > 0 || quota.value === null) {
      this.updateWarnings = warnings;
      this.updateErrorMessage = warnings[0] ?? 'Please check the form fields.';
      return null;
    }

    return {
      username,
      email,
      fullName,
      storageQuotaBytes: quota.value
    };
  }

  private syncFriendlyToBytes(form: { storageQuotaInput: string; storageQuotaBytes: string }): void {
    const raw = form.storageQuotaInput.trim();
    if (raw === '') {
      form.storageQuotaBytes = '';
      return;
    }

    const parsed = this.parseStorageSize(raw);
    if (parsed === null) {
      return;
    }

    form.storageQuotaBytes = String(parsed);
  }

  private syncBytesToFriendly(form: { storageQuotaInput: string; storageQuotaBytes: string }): void {
    const parsed = this.parseBytesField(form.storageQuotaBytes);
    if (parsed === null) {
      return;
    }

    form.storageQuotaInput = this.formatBytes(parsed);
  }

  private resolveQuotaBytesFromFields(
    friendlyRawValue: string,
    bytesRawValue: string
  ): { value: number | null; error?: string } {
    const friendlyRaw = friendlyRawValue.trim();
    const bytesRaw = bytesRawValue.trim();

    if (friendlyRaw === '' && bytesRaw === '') {
      return { value: null, error: 'Storage quota is required.' };
    }

    const friendlyBytes = friendlyRaw === '' ? null : this.parseStorageSize(friendlyRaw);
    if (friendlyRaw !== '' && friendlyBytes === null) {
      return {
        value: null,
        error: 'Invalid quota size format. Use values like 500GB, 1.5TB, 800GiB, or input bytes.'
      };
    }

    const bytesValue = bytesRaw === '' ? null : this.parseBytesField(bytesRaw);
    if (bytesRaw !== '' && bytesValue === null) {
      return { value: null, error: 'Quota bytes must be a non-negative integer.' };
    }

    if (friendlyBytes !== null && bytesValue !== null && friendlyBytes !== bytesValue) {
      return { value: null, error: 'Quota size and bytes input do not match.' };
    }

    const finalValue = bytesValue ?? friendlyBytes;
    if (finalValue === null) {
      return { value: null, error: 'Storage quota is required.' };
    }

    return { value: finalValue };
  }

  private parseBytesField(raw: string): number | null {
    const trimmed = raw.trim();
    if (trimmed === '') {
      return null;
    }

    if (!/^\d+$/.test(trimmed)) {
      return null;
    }

    const value = Number(trimmed);
    if (!Number.isSafeInteger(value) || value < 0) {
      return null;
    }

    return value;
  }

  private parseStorageSize(raw: string): number | null {
    const match = raw.trim().toUpperCase().match(/^(\d+(?:\.\d+)?)\s*([A-Z]{0,3})$/);
    if (!match) {
      return null;
    }

    const value = Number(match[1]);
    if (!Number.isFinite(value) || value < 0) {
      return null;
    }

    const unit = match[2] === '' ? 'B' : match[2];
    const multipliers: Record<string, number> = {
      B: 1,
      KB: 1_000,
      MB: 1_000_000,
      GB: 1_000_000_000,
      TB: 1_000_000_000_000,
      PB: 1_000_000_000_000_000,
      KIB: 1_024,
      MIB: 1_048_576,
      GIB: 1_073_741_824,
      TIB: 1_099_511_627_776,
      PIB: 1_125_899_906_842_624
    };

    const multiplier = multipliers[unit];
    if (!multiplier) {
      return null;
    }

    const bytes = Math.round(value * multiplier);
    if (!Number.isSafeInteger(bytes) || bytes < 0) {
      return null;
    }

    return bytes;
  }

  private formatBytes(bytes: number): string {
    const units = ['B', 'KB', 'MB', 'GB', 'TB', 'PB'];
    let value = bytes;
    let idx = 0;

    while (value >= 1000 && idx < units.length - 1) {
      value /= 1000;
      idx += 1;
    }

    if (idx === 0) {
      return `${Math.round(value)} ${units[idx]}`;
    }

    const rounded = value >= 100 ? value.toFixed(0) : value >= 10 ? value.toFixed(1) : value.toFixed(2);
    const compact = rounded.replace(/\.0+$/, '').replace(/(\.\d*[1-9])0+$/, '$1');
    return `${compact} ${units[idx]}`;
  }

  private looksLikeEmail(value: string): boolean {
    const at = value.indexOf('@');
    if (at <= 0 || at === value.length - 1) {
      return false;
    }

    const domain = value.slice(at + 1);
    return domain.includes('.');
  }

  private extractError(payload: unknown, fallback: string): string {
    if (payload instanceof HttpErrorResponse) {
      if (
        payload.error &&
        typeof payload.error === 'object' &&
        'error' in payload.error &&
        typeof (payload.error as ApiErrorResponseDto).error === 'string'
      ) {
        return (payload.error as ApiErrorResponseDto).error;
      }

      if (typeof payload.error === 'string' && payload.error.length > 0) {
        return payload.error;
      }

      if (payload.message) {
        return payload.message;
      }

      return fallback;
    }

    if (
      payload &&
      typeof payload === 'object' &&
      'error' in payload &&
      typeof (payload as { error?: unknown }).error === 'string'
    ) {
      return (payload as { error: string }).error;
    }

    return fallback;
  }
}
