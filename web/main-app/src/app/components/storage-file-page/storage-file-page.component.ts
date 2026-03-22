import { CommonModule } from '@angular/common';
import { HttpErrorResponse } from '@angular/common/http';
import { ChangeDetectorRef, Component, OnDestroy, OnInit } from '@angular/core';
import { ActivatedRoute, Router } from '@angular/router';
import { Subscription } from 'rxjs';

import { ApiErrorResponseDto } from '../../dto/api-error-response.dto';
import { StorageFileMetadataDto } from '../../dto/storage-file-metadata.dto';
import { ClientSessionService } from '../../services/client-session.service';
import { StorageApiService } from '../../services/storage-api.service';

@Component({
  selector: 'app-storage-file-page',
  imports: [CommonModule],
  templateUrl: './storage-file-page.component.html',
  styleUrl: './storage-file-page.component.css'
})
export class StorageFilePageComponent implements OnInit, OnDestroy {
  private routeSub: Subscription | null = null;
  private metadataSub: Subscription | null = null;

  loading = false;
  errorMessage = '';
  metadata: StorageFileMetadataDto | null = null;

  constructor(
    private readonly route: ActivatedRoute,
    private readonly router: Router,
    private readonly sessionService: ClientSessionService,
    private readonly storageApiService: StorageApiService,
    private readonly cdr: ChangeDetectorRef
  ) {}

  ngOnInit(): void {
    this.routeSub = this.route.paramMap.subscribe((params) => {
      const rawId = params.get('fileId') ?? '';
      const fileId = Number(rawId);

      if (!Number.isInteger(fileId) || fileId <= 0) {
        this.errorMessage = 'Invalid file route';
        this.metadata = null;
        this.loading = false;
        this.cdr.detectChanges();
        return;
      }

      this.loadMetadata(fileId);
    });
  }

  ngOnDestroy(): void {
    this.routeSub?.unsubscribe();
    this.routeSub = null;
    this.metadataSub?.unsubscribe();
    this.metadataSub = null;
  }

  openContainingFolder(): void {
    if (!this.metadata) {
      return;
    }

    if (this.isSharedSource()) {
      this.router.navigate(['/app/storage/folders', this.metadata.folderId], {
        queryParams: { source: 'shared' }
      });
      return;
    }

    this.router.navigate(['/app/storage/folders', this.metadata.folderId]);
  }

  backToStorage(): void {
    const folderId = this.metadata?.folderId;

    if (folderId && Number.isInteger(folderId) && folderId > 0) {
      if (this.isSharedSource()) {
        this.router.navigate(['/app/storage/folders', folderId], {
          queryParams: { source: 'shared' }
        });
        return;
      }

      this.router.navigate(['/app/storage/folders', folderId]);
      return;
    }

    if (this.isSharedSource()) {
      this.router.navigate(['/app/shared']);
      return;
    }

    this.router.navigate(['/app/storage']);
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
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit'
    }).format(date);
  }

  private loadMetadata(fileId: number): void {
    const accessToken = this.sessionService.readAccessToken();
    if (!accessToken) {
      this.redirectToLogin();
      return;
    }

    this.metadataSub?.unsubscribe();
    this.loading = true;
    this.errorMessage = '';
    this.metadata = null;

    this.metadataSub = this.storageApiService
      .fileMetadata('', accessToken, fileId)
      .subscribe({
        next: (payload) => {
          this.metadata = payload;
          this.loading = false;
          this.cdr.detectChanges();
        },
        error: (error: unknown) => {
          this.loading = false;
          this.errorMessage = this.extractError(error, 'Failed to load file metadata');

          if (error instanceof HttpErrorResponse && error.status === 401) {
            this.redirectToLogin();
            return;
          }

          this.cdr.detectChanges();
        }
      });
  }

  private redirectToLogin(): void {
    this.sessionService.clearSession();
    this.router.navigate(['/login']);
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
    }

    return fallback;
  }

  private isSharedSource(): boolean {
    return this.route.snapshot.queryParamMap.get('source') === 'shared';
  }
}
