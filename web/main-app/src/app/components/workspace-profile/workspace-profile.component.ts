import { CommonModule } from '@angular/common';
import { ChangeDetectorRef, Component, OnInit } from '@angular/core';
import { Router } from '@angular/router';

import { AuthUserDto } from '../../dto/auth-user.dto';
import { AuthApiService } from '../../services/auth-api.service';
import { ClientSessionService } from '../../services/client-session.service';

@Component({
  selector: 'app-workspace-profile',
  imports: [CommonModule],
  templateUrl: './workspace-profile.component.html',
  styleUrl: './workspace-profile.component.css'
})
export class WorkspaceProfileComponent implements OnInit {
  loading = true;
  errorMessage = '';
  currentUser: AuthUserDto | null = null;

  constructor(
    private readonly authApiService: AuthApiService,
    private readonly sessionService: ClientSessionService,
    private readonly router: Router,
    private readonly cdr: ChangeDetectorRef
  ) {}

  ngOnInit(): void {
    this.loadProfile();
  }

  signOut(): void {
    const token = this.sessionService.readAccessToken();

    if (!token) {
      this.sessionService.clearSession();
      this.router.navigate(['/login']);
      return;
    }

    this.authApiService.logout('', token).subscribe({
      next: () => {
        this.sessionService.clearSession();
        this.router.navigate(['/login']);
      },
      error: () => {
        this.sessionService.clearSession();
        this.router.navigate(['/login']);
      }
    });
  }

  formatSize(bytes: number | null): string {
    if (bytes === null || !Number.isFinite(bytes) || bytes < 0) {
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

  private loadProfile(): void {
    const token = this.sessionService.readAccessToken();

    if (!token) {
      this.sessionService.clearSession();
      this.router.navigate(['/login']);
      return;
    }

    this.authApiService.me('', token).subscribe({
      next: (response) => {
        this.currentUser = response.user;
        this.loading = false;
        this.cdr.detectChanges();
      },
      error: () => {
        this.errorMessage = 'Failed to load profile';
        this.loading = false;
        this.cdr.detectChanges();
      }
    });
  }
}
