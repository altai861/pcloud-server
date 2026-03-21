import { CommonModule } from '@angular/common';
import { HttpErrorResponse } from '@angular/common/http';
import { ChangeDetectorRef, Component, OnInit } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { finalize } from 'rxjs';
import { ApiErrorResponseDto } from './dto/api-error-response.dto';
import { SetupInitializeRequestDto } from './dto/setup-initialize-request.dto';
import { SetupApiService } from './services/setup-api.service';
import { ThemeService } from './services/theme.service';

@Component({
  selector: 'app-root',
  imports: [CommonModule, FormsModule],
  templateUrl: './app.html',
  styleUrl: './app.css'
})
export class App implements OnInit {
  loading = true;
  submitting = false;
  isInitialized = false;
  errorMessage = '';
  successMessage = '';
  isDarkMode = false;

  form = {
    admin: {
      username: 'admin',
      email: 'admin@example.com',
      fullName: 'System Administrator',
      password: '',
      passwordConfirmation: ''
    },
    system: {
      storageRootPath: '/srv/pcloud-storage',
      totalStorageLimitInput: '500GB',
      totalStorageLimitBytes: '500000000000'
    }
  };

  constructor(
    private readonly setupApiService: SetupApiService,
    private readonly themeService: ThemeService,
    private readonly cdr: ChangeDetectorRef
  ) { }

  ngOnInit(): void {
    this.isDarkMode = this.themeService.initializeTheme() === 'dark';
    this.onFriendlyLimitChange();
    this.loadSetupStatus();
  }

  toggleTheme(): void {
    this.isDarkMode = this.themeService.toggleTheme() === 'dark';
  }

  submit(): void {
    if (this.submitting || this.loading) {
      return;
    }

    this.errorMessage = '';
    this.successMessage = '';

    const clientValidationError = this.validateClientInput();
    if (clientValidationError) {
      this.errorMessage = clientValidationError;
      return;
    }

    const totalLimitResult = this.resolveTotalLimitBytesFromForm();
    if (totalLimitResult.error) {
      this.errorMessage = totalLimitResult.error;
      return;
    }

    const payload: SetupInitializeRequestDto = {
      admin: {
        username: this.form.admin.username.trim(),
        email: this.form.admin.email.trim(),
        fullName: this.form.admin.fullName.trim(),
        password: this.form.admin.password,
        passwordConfirmation: this.form.admin.passwordConfirmation
      },
      system: {
        storageRootPath: this.form.system.storageRootPath.trim(),
        totalStorageLimitBytes: totalLimitResult.value
      }
    };

    this.submitting = true;

    this.setupApiService
      .initializeSetup(payload)
      .pipe(finalize(() => {
        this.submitting = false;
        this.cdr.detectChanges();
      }))
      .subscribe({
        next: (result) => {
          this.successMessage = result.message;
          this.isInitialized = result.isInitialized;
          this.cdr.detectChanges();
        },
        error: (error: unknown) => {
          this.errorMessage = this.extractError(error, 'Setup initialization failed');
          this.cdr.detectChanges();
        }
      });
  }

  private loadSetupStatus(): void {
    this.loading = true;
    this.errorMessage = '';

    this.setupApiService
      .getSetupStatus()
      .subscribe({
        next: (body) => {
          this.isInitialized = body.isInitialized;
          this.cdr.detectChanges();
        },
        error: (error: unknown) => {
          this.errorMessage = this.extractError(error, 'Failed to load setup status');
          this.cdr.detectChanges();
        },
        complete: () => {
          this.loading = false;
          this.cdr.detectChanges();
        }
      });
  }

  private validateClientInput(): string | null {
    if (this.form.admin.password !== this.form.admin.passwordConfirmation) {
      return 'Password and confirmation must match.';
    }

    const rootPath = this.form.system.storageRootPath.trim();
    if (!rootPath.startsWith('/')) {
      return 'Storage root path must be absolute.';
    }

    const totalLimitResult = this.resolveTotalLimitBytesFromForm();
    if (totalLimitResult.error) {
      return totalLimitResult.error;
    }

    return null;
  }

  onFriendlyLimitChange(): void {
    const raw = this.form.system.totalStorageLimitInput.trim();
    if (raw === '') {
      this.form.system.totalStorageLimitBytes = '';
      return;
    }

    const parsed = this.parseStorageSize(raw);
    if (parsed === null) {
      return;
    }

    this.form.system.totalStorageLimitBytes = String(parsed);
  }

  onBytesLimitChange(): void {
    const bytes = this.parseBytesField(this.form.system.totalStorageLimitBytes);
    if (bytes === null) {
      return;
    }

    this.form.system.totalStorageLimitInput = this.formatBytes(bytes);
  }

  private resolveTotalLimitBytesFromForm(): { value: number | null; error?: string } {
    const friendlyRaw = this.form.system.totalStorageLimitInput.trim();
    const bytesRaw = this.form.system.totalStorageLimitBytes.trim();

    if (friendlyRaw === '' && bytesRaw === '') {
      return { value: null };
    }

    const friendlyBytes = friendlyRaw === '' ? null : this.parseStorageSize(friendlyRaw);
    if (friendlyRaw !== '' && friendlyBytes === null) {
      return {
        value: null,
        error: 'Invalid size format. Use values like 500GB, 1.5TB, 800GiB, or input bytes.'
      };
    }

    const bytesValue = bytesRaw === '' ? null : this.parseBytesField(bytesRaw);
    if (bytesRaw !== '' && bytesValue === null) {
      return {
        value: null,
        error: 'Bytes must be a positive integer.'
      };
    }

    if (friendlyBytes !== null && bytesValue !== null && friendlyBytes !== bytesValue) {
      return {
        value: null,
        error: 'Friendly size and bytes input do not match.'
      };
    }

    return { value: bytesValue ?? friendlyBytes ?? null };
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
    if (!Number.isSafeInteger(value) || value <= 0) {
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
    if (!Number.isFinite(value) || value <= 0) {
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
    if (!Number.isSafeInteger(bytes) || bytes <= 0) {
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
