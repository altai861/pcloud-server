import { CommonModule } from '@angular/common';
import { HttpErrorResponse } from '@angular/common/http';
import { ChangeDetectorRef, Component, OnInit } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { Router } from '@angular/router';
import { finalize, timeout } from 'rxjs';

import { ApiErrorResponseDto } from '../../dto/api-error-response.dto';
import { LoginRequestDto } from '../../dto/login-request.dto';
import { LoginFormModel } from '../../models/login-form.model';
import { TPipe } from '../../pipes/t.pipe';
import { AuthApiService } from '../../services/auth-api.service';
import { ClientSessionService } from '../../services/client-session.service';
import { I18nService } from '../../services/i18n.service';
import { ThemeService } from '../../services/theme.service';

@Component({
  selector: 'app-login-page',
  imports: [CommonModule, FormsModule, TPipe],
  templateUrl: './login-page.component.html',
  styleUrl: './login-page.component.css'
})
export class LoginPageComponent implements OnInit {
  checkingStatus = true;
  isInitialized = false;
  loginSubmitting = false;
  loginErrorMessage = '';
  isDarkMode = false;
  currentLanguage: 'en' | 'mn' = 'en';

  form: LoginFormModel = {
    username: '',
    password: ''
  };

  constructor(
    private readonly authApiService: AuthApiService,
    private readonly sessionService: ClientSessionService,
    private readonly i18nService: I18nService,
    private readonly themeService: ThemeService,
    private readonly router: Router,
    private readonly cdr: ChangeDetectorRef
  ) {}

  ngOnInit(): void {
    this.currentLanguage = this.i18nService.initializeLanguage();
    this.isDarkMode = this.themeService.initializeTheme() === 'dark';
    this.loadStatusAndBootstrapSession();
  }

  toggleTheme(): void {
    this.isDarkMode = this.themeService.toggleTheme() === 'dark';
  }

  toggleLanguage(): void {
    this.currentLanguage = this.i18nService.toggleLanguage();
    this.cdr.detectChanges();
  }

  submit(): void {
    if (this.loginSubmitting || this.checkingStatus) {
      return;
    }

    this.loginErrorMessage = '';

    const payload = this.resolveLoginPayload();
    if (!payload) {
      return;
    }

    this.loginSubmitting = true;

    this.authApiService
      .login('', payload)
      .pipe(timeout(12_000))
      .pipe(finalize(() => {
        this.loginSubmitting = false;
        this.cdr.detectChanges();
      }))
      .subscribe({
        next: (body) => {
          this.sessionService.setSession(body.accessToken);
          this.router.navigate(['/app/storage']);
        },
        error: (error: unknown) => {
          this.loginErrorMessage = this.extractError(error, this.i18nService.t('login.error.loginFailed'));
          this.cdr.detectChanges();
        }
      });
  }

  private loadStatusAndBootstrapSession(): void {
    this.checkingStatus = true;
    this.loginErrorMessage = '';

    this.authApiService
      .getClientStatus('')
      .pipe(timeout(8_000))
      .subscribe({
        next: (status) => {
          this.isInitialized = status.isInitialized;

          if (!status.isInitialized) {
            this.loginErrorMessage = this.i18nService.t('login.error.setupNotCompleted');
            this.checkingStatus = false;
            this.cdr.detectChanges();
            return;
          }

          const token = this.sessionService.readAccessToken();
          if (!token) {
            this.checkingStatus = false;
            this.cdr.detectChanges();
            return;
          }

          this.restoreSession(token);
        },
        error: (error: unknown) => {
          this.loginErrorMessage = this.extractError(error, this.i18nService.t('login.error.connectFailed'));
          this.checkingStatus = false;
          this.cdr.detectChanges();
        }
      });
  }

  private restoreSession(accessToken: string): void {
    this.authApiService
      .me('', accessToken)
      .pipe(timeout(8_000))
      .pipe(finalize(() => {
        this.checkingStatus = false;
        this.cdr.detectChanges();
      }))
      .subscribe({
        next: () => {
          this.router.navigate(['/app/storage']);
        },
        error: () => {
          this.sessionService.clearSession();
          this.loginErrorMessage = this.i18nService.t('login.error.sessionExpired');
          this.cdr.detectChanges();
        }
      });
  }

  private resolveLoginPayload(): LoginRequestDto | null {
    const username = this.form.username.trim();
    const password = this.form.password;

    if (username.length === 0) {
      this.loginErrorMessage = this.i18nService.t('login.error.usernameRequired');
      return null;
    }

    if (password.length === 0) {
      this.loginErrorMessage = this.i18nService.t('login.error.passwordRequired');
      return null;
    }

    return {
      username,
      password
    };
  }

  private extractError(payload: unknown, fallback: string): string {
    if (
      payload &&
      typeof payload === 'object' &&
      'name' in payload &&
      (payload as { name?: unknown }).name === 'TimeoutError'
    ) {
      return this.i18nService.t('login.error.timeout');
    }

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
