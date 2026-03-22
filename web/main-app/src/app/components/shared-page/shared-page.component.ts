import { CommonModule } from '@angular/common';
import { HttpErrorResponse } from '@angular/common/http';
import { ChangeDetectorRef, Component, HostListener, OnDestroy, OnInit } from '@angular/core';
import { Router } from '@angular/router';
import { finalize, Subscription } from 'rxjs';

import { ApiErrorResponseDto } from '../../dto/api-error-response.dto';
import { SharedResourceEntryDto } from '../../dto/shared-resource-entry.dto';
import { ClientSessionService } from '../../services/client-session.service';
import { StorageApiService } from '../../services/storage-api.service';

@Component({
  selector: 'app-shared-page',
  imports: [CommonModule],
  templateUrl: './shared-page.component.html',
  styleUrl: './shared-page.component.css'
})
export class SharedPageComponent implements OnInit, OnDestroy {
  private loadSub: Subscription | null = null;
  private ownerAvatarUrls = new Map<number, string>();
  private ownerAvatarFallbackIds = new Set<number>();
  private ownerAvatarAccessToken: string | null = null;

  entries: SharedResourceEntryDto[] = [];
  loading = false;
  errorMessage = '';

  isEntryMenuOpen = false;
  entryMenuX = 0;
  entryMenuY = 0;
  entryMenuTarget: SharedResourceEntryDto | null = null;

  constructor(
    private readonly storageApiService: StorageApiService,
    private readonly sessionService: ClientSessionService,
    private readonly router: Router,
    private readonly cdr: ChangeDetectorRef
  ) {}

  ngOnInit(): void {
    this.loadSharedResources();
  }

  ngOnDestroy(): void {
    this.loadSub?.unsubscribe();
    this.loadSub = null;
  }

  openEntry(entry: SharedResourceEntryDto): void {
    if (entry.resourceType === 'folder') {
      this.router.navigate(['/app/storage/folders', entry.resourceId], {
        queryParams: { source: 'shared' }
      });
      return;
    }

    this.router.navigate(['/app/storage/files', entry.resourceId], {
      queryParams: { source: 'shared' }
    });
  }

  openEntryMenuFromButton(event: MouseEvent, entry: SharedResourceEntryDto): void {
    event.preventDefault();
    event.stopPropagation();
    this.openEntryMenuAt(event.clientX, event.clientY, entry);
  }

  openEntryContextMenu(event: MouseEvent, entry: SharedResourceEntryDto): void {
    event.preventDefault();
    event.stopPropagation();
    this.openEntryMenuAt(event.clientX, event.clientY, entry);
  }

  closeEntryMenu(): void {
    this.isEntryMenuOpen = false;
    this.entryMenuTarget = null;
  }

  onOpenResourceClick(event: MouseEvent): void {
    event.preventDefault();
    event.stopPropagation();

    const selectedEntry = this.entryMenuTarget;
    if (!selectedEntry) {
      return;
    }

    this.closeEntryMenu();
    this.openEntry(selectedEntry);
  }

  onDownloadResourceClick(event: MouseEvent): void {
    event.preventDefault();
    event.stopPropagation();

    const selectedEntry = this.entryMenuTarget;
    if (!selectedEntry || selectedEntry.resourceType !== 'file') {
      return;
    }

    const accessToken = this.sessionService.readAccessToken();
    if (!accessToken) {
      this.redirectToLogin();
      return;
    }

    this.closeEntryMenu();

    const downloadUrl = this.storageApiService.buildFileDownloadUrl(
      '',
      accessToken,
      selectedEntry.path,
      selectedEntry.resourceId
    );

    const anchor = document.createElement('a');
    anchor.href = downloadUrl;
    anchor.target = '_blank';
    anchor.rel = 'noopener noreferrer';
    document.body.appendChild(anchor);
    anchor.click();
    anchor.remove();
  }

  get isEntryDownloadDisabled(): boolean {
    return !this.entryMenuTarget || this.entryMenuTarget.resourceType !== 'file';
  }

  ownerAvatarSrc(ownerUserId: number): string {
    if (this.ownerAvatarFallbackIds.has(ownerUserId)) {
      return 'profile.png';
    }

    const cached = this.ownerAvatarUrls.get(ownerUserId);
    if (cached) {
      return cached;
    }

    const accessToken = this.ownerAvatarAccessToken ?? this.sessionService.readAccessToken();
    if (!accessToken) {
      return 'profile.png';
    }

    this.ownerAvatarAccessToken = accessToken;
    const src = this.storageApiService.buildUserProfileImageUrl('', accessToken, ownerUserId);
    this.ownerAvatarUrls.set(ownerUserId, src);
    return src;
  }

  onOwnerAvatarError(event: Event, ownerUserId: number): void {
    this.ownerAvatarFallbackIds.add(ownerUserId);

    const image = event.target as HTMLImageElement | null;
    if (image) {
      image.src = 'profile.png';
    }
  }

  isDefaultOwnerAvatar(ownerUserId: number): boolean {
    return this.ownerAvatarFallbackIds.has(ownerUserId);
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

  @HostListener('document:click')
  onDocumentClick(): void {
    if (!this.isEntryMenuOpen) {
      return;
    }

    this.closeEntryMenu();
    this.cdr.detectChanges();
  }

  @HostListener('document:keydown.escape')
  onEscapePressed(): void {
    if (!this.isEntryMenuOpen) {
      return;
    }

    this.closeEntryMenu();
    this.cdr.detectChanges();
  }

  private loadSharedResources(): void {
    const accessToken = this.sessionService.readAccessToken();
    if (!accessToken) {
      this.redirectToLogin();
      return;
    }

    this.loadSub?.unsubscribe();
    this.loading = true;
    this.errorMessage = '';

    this.loadSub = this.storageApiService
      .listShared('', accessToken, '')
      .pipe(
        finalize(() => {
          this.loading = false;
          this.cdr.detectChanges();
        })
      )
      .subscribe({
        next: (response) => {
          this.entries = response.entries;
          this.ownerAvatarAccessToken = accessToken;
          this.cdr.detectChanges();
        },
        error: (error: unknown) => {
          this.errorMessage = this.extractError(error, 'Failed to load shared resources');

          if (error instanceof HttpErrorResponse && error.status === 401) {
            this.redirectToLogin();
            return;
          }

          this.cdr.detectChanges();
        }
      });
  }

  private openEntryMenuAt(clientX: number, clientY: number, entry: SharedResourceEntryDto): void {
    const menuWidth = 184;
    const menuHeight = 120;
    const viewportPadding = 8;

    const left = Math.max(
      viewportPadding,
      Math.min(clientX, window.innerWidth - menuWidth - viewportPadding)
    );
    const top = Math.max(
      viewportPadding,
      Math.min(clientY, window.innerHeight - menuHeight - viewportPadding)
    );

    this.entryMenuX = left;
    this.entryMenuY = top;
    this.entryMenuTarget = entry;
    this.isEntryMenuOpen = true;
    this.cdr.detectChanges();
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
}
