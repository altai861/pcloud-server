import { CommonModule } from '@angular/common';
import { ChangeDetectorRef, Component, OnDestroy, OnInit } from '@angular/core';
import { Router } from '@angular/router';
import { Subscription } from 'rxjs';

import { AuthUserDto } from '../../dto/auth-user.dto';
import { AuthApiService } from '../../services/auth-api.service';
import { ClientSessionService } from '../../services/client-session.service';
import { ProfileImageService } from '../../services/profile-image.service';

@Component({
  selector: 'app-workspace-profile',
  imports: [CommonModule],
  templateUrl: './workspace-profile.component.html',
  styleUrl: './workspace-profile.component.css'
})
export class WorkspaceProfileComponent implements OnInit, OnDestroy {
  private profileImageSub: Subscription | null = null;

  loading = true;
  uploadingImage = false;
  errorMessage = '';
  imageErrorMessage = '';
  imageSuccessMessage = '';
  currentUser: AuthUserDto | null = null;
  profileImageSrc: string | null = null;

  constructor(
    private readonly authApiService: AuthApiService,
    private readonly sessionService: ClientSessionService,
    private readonly profileImageService: ProfileImageService,
    private readonly router: Router,
    private readonly cdr: ChangeDetectorRef
  ) {}

  ngOnInit(): void {
    this.profileImageSub = this.profileImageService.profileImageSrc$.subscribe((src) => {
      this.profileImageSrc = src;
      this.cdr.detectChanges();
    });
    this.loadProfile();
  }

  ngOnDestroy(): void {
    this.profileImageSub?.unsubscribe();
    this.profileImageSub = null;
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
        this.profileImageService.clear();
        this.sessionService.clearSession();
        this.router.navigate(['/login']);
      },
      error: () => {
        this.profileImageService.clear();
        this.sessionService.clearSession();
        this.router.navigate(['/login']);
      }
    });
  }

  onProfileImageSelected(event: Event): void {
    const input = event.target as HTMLInputElement;
    const selectedFile = input.files?.[0] ?? null;

    this.imageErrorMessage = '';
    this.imageSuccessMessage = '';

    if (!selectedFile) {
      return;
    }

    if (!selectedFile.type.startsWith('image/')) {
      this.imageErrorMessage = 'Please select an image file.';
      return;
    }

    this.uploadProfileImage(selectedFile);
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
        if (response.user.profileImageUrl) {
          this.loadProfileImage(token);
        } else {
          this.profileImageService.clear();
        }
        this.loading = false;
        this.cdr.detectChanges();
      },
      error: () => {
        this.errorMessage = 'Failed to load profile';
        this.profileImageService.clear();
        this.loading = false;
        this.cdr.detectChanges();
      }
    });
  }

  private uploadProfileImage(imageFile: File): void {
    const token = this.sessionService.readAccessToken();
    if (!token) {
      this.sessionService.clearSession();
      this.router.navigate(['/login']);
      return;
    }

    this.uploadingImage = true;
    this.imageErrorMessage = '';
    this.imageSuccessMessage = '';

    this.authApiService.updateProfileImage('', token, imageFile).subscribe({
      next: (response) => {
        this.currentUser = response.user;
        this.imageSuccessMessage = response.message;
        this.loadProfileImage(token);
        this.uploadingImage = false;
        this.cdr.detectChanges();
      },
      error: () => {
        this.imageErrorMessage = 'Failed to upload profile image.';
        this.uploadingImage = false;
        this.cdr.detectChanges();
      }
    });
  }

  private loadProfileImage(token: string): void {
    this.authApiService.getProfileImage('', token).subscribe({
      next: (blob) => {
        this.profileImageService.setFromBlob(blob);
      },
      error: () => {
        this.profileImageService.clear();
      }
    });
  }
}
