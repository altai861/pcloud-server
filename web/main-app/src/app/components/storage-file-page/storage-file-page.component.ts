import { CommonModule } from '@angular/common';
import { HttpErrorResponse } from '@angular/common/http';
import { ChangeDetectorRef, Component, OnDestroy, OnInit, ViewRef } from '@angular/core';
import { ActivatedRoute, Router } from '@angular/router';
import { PdfViewerModule } from 'ng2-pdf-viewer';
import { Subscription } from 'rxjs';

import { ApiErrorResponseDto } from '../../dto/api-error-response.dto';
import { TPipe } from '../../pipes/t.pipe';
import { StorageFileMetadataDto } from '../../dto/storage-file-metadata.dto';
import { ClientSessionService } from '../../services/client-session.service';
import { StorageApiService } from '../../services/storage-api.service';

@Component({
  selector: 'app-storage-file-page',
  imports: [CommonModule, TPipe, PdfViewerModule],
  templateUrl: './storage-file-page.component.html',
  styleUrl: './storage-file-page.component.css'
})
export class StorageFilePageComponent implements OnInit, OnDestroy {
  private static readonly IMAGE_EXTENSIONS = new Set([
    'jpg', 'jpeg', 'png', 'gif', 'webp', 'bmp', 'svg', 'heic', 'heif', 'tif', 'tiff', 'ico', 'avif'
  ]);
  private static readonly TEXT_EXTENSIONS = new Set([
    'txt', 'md', 'markdown', 'json', 'xml', 'yaml', 'yml', 'toml', 'csv', 'tsv', 'log', 'ini', 'conf',
    'rs', 'ts', 'tsx', 'js', 'jsx', 'mjs', 'cjs', 'html', 'css', 'scss', 'less', 'go', 'java', 'py',
    'c', 'cpp', 'h', 'hpp', 'php', 'sh', 'sql'
  ]);
  private static readonly AUDIO_EXTENSIONS = new Set([
    'mp3', 'wav', 'ogg', 'flac', 'm4a', 'aac'
  ]);
  private static readonly VIDEO_EXTENSIONS = new Set([
    'mp4', 'webm', 'mov', 'm4v', 'ogv'
  ]);
  private static readonly TEXT_MIME_TYPES = new Set([
    'application/json',
    'application/xml',
    'application/x-yaml',
    'application/yaml',
    'application/toml',
    'application/javascript',
    'application/x-javascript',
    'application/typescript',
    'application/sql',
    'image/svg+xml'
  ]);
  private static readonly MAX_TEXT_PREVIEW_BYTES = 5 * 1024 * 1024;

  private routeSub: Subscription | null = null;
  private metadataSub: Subscription | null = null;
  private previewSub: Subscription | null = null;

  loading = false;
  errorMessage = '';
  metadata: StorageFileMetadataDto | null = null;
  previewMode: 'none' | 'image' | 'pdf' | 'text' | 'audio' | 'video' | 'unsupported' = 'none';
  previewLoading = false;
  previewErrorMessage = '';
  previewText = '';
  previewUrl: string | null = null;
  previewUnsupportedReasonKey: string | null = null;
  isMetadataModalOpen = false;

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
        this.requestViewRefresh();
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
    this.previewSub?.unsubscribe();
    this.previewSub = null;
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

  openMetadataModal(): void {
    if (!this.metadata) {
      return;
    }

    this.isMetadataModalOpen = true;
  }

  closeMetadataModal(): void {
    this.isMetadataModalOpen = false;
  }

  private loadMetadata(fileId: number): void {
    const accessToken = this.sessionService.readAccessToken();
    if (!accessToken) {
      this.redirectToLogin();
      return;
    }

    this.metadataSub?.unsubscribe();
    this.previewSub?.unsubscribe();
    this.loading = true;
    this.errorMessage = '';
    this.metadata = null;
    this.isMetadataModalOpen = false;
    this.resetPreviewState();

    this.metadataSub = this.storageApiService
      .fileMetadata('', accessToken, fileId)
      .subscribe({
        next: (payload) => {
          this.metadata = payload;
          this.loading = false;
          this.loadPreview(fileId, payload, accessToken);
          this.requestViewRefresh();
        },
        error: (error: unknown) => {
          this.loading = false;
          this.errorMessage = this.extractError(error, 'Failed to load file metadata');

          if (error instanceof HttpErrorResponse && error.status === 401) {
            this.redirectToLogin();
            return;
          }

          this.requestViewRefresh();
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

  private loadPreview(
    fileId: number,
    metadata: StorageFileMetadataDto,
    accessToken: string
  ): void {
    this.previewSub?.unsubscribe();
    this.resetPreviewState();

    const previewMode = this.resolvePreviewMode(metadata);
    this.previewMode = previewMode;

    if (previewMode === 'unsupported') {
      this.previewUnsupportedReasonKey = 'fileMeta.previewUnsupported';
      this.requestViewRefresh();
      return;
    }

    if (previewMode === 'text' && metadata.sizeBytes > StorageFilePageComponent.MAX_TEXT_PREVIEW_BYTES) {
      this.previewMode = 'unsupported';
      this.previewUnsupportedReasonKey = 'fileMeta.previewTooLarge';
      this.requestViewRefresh();
      return;
    }

    this.previewLoading = true;

    if (previewMode === 'image' || previewMode === 'pdf' || previewMode === 'audio' || previewMode === 'video') {
      this.previewUrl = this.storageApiService.buildFilePreviewUrl('', accessToken, fileId);
      this.previewLoading = false;
      this.requestViewRefresh();
      return;
    }

    this.previewSub = this.storageApiService.previewTextFile('', accessToken, fileId).subscribe({
      next: (content) => {
        this.previewText = content;
        this.previewLoading = false;
        this.requestViewRefresh();
      },
      error: (error: unknown) => {
        this.previewLoading = false;
        this.previewErrorMessage = this.extractError(error, 'Failed to load file preview');
        this.requestViewRefresh();
      }
    });
  }

  private resolvePreviewMode(
    metadata: StorageFileMetadataDto
  ): 'image' | 'pdf' | 'text' | 'audio' | 'video' | 'unsupported' {
    const mimeType = (metadata.mimeType ?? '').trim().toLowerCase();
    const extension = (metadata.extension ?? '').trim().replace(/^\./, '').toLowerCase();

    if (mimeType === 'application/pdf' || extension === 'pdf') {
      return 'pdf';
    }

    if (mimeType.startsWith('image/') || StorageFilePageComponent.IMAGE_EXTENSIONS.has(extension)) {
      return 'image';
    }

    if (
      mimeType.startsWith('text/') ||
      StorageFilePageComponent.TEXT_MIME_TYPES.has(mimeType) ||
      StorageFilePageComponent.TEXT_EXTENSIONS.has(extension)
    ) {
      return 'text';
    }

    if (mimeType.startsWith('audio/') || StorageFilePageComponent.AUDIO_EXTENSIONS.has(extension)) {
      return 'audio';
    }

    if (mimeType.startsWith('video/') || StorageFilePageComponent.VIDEO_EXTENSIONS.has(extension)) {
      return 'video';
    }

    return 'unsupported';
  }

  private resetPreviewState(): void {
    this.previewMode = 'none';
    this.previewLoading = false;
    this.previewErrorMessage = '';
    this.previewText = '';
    this.previewUrl = null;
    this.previewUnsupportedReasonKey = null;
  }

  private requestViewRefresh(): void {
    queueMicrotask(() => {
      const viewRef = this.cdr as ViewRef;
      if (viewRef.destroyed) {
        return;
      }

      this.cdr.detectChanges();
    });
  }
}
