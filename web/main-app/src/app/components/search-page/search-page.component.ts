import { CommonModule } from '@angular/common';
import { HttpErrorResponse } from '@angular/common/http';
import { ChangeDetectorRef, Component, OnDestroy, OnInit } from '@angular/core';
import { ActivatedRoute, Router } from '@angular/router';
import { finalize, Subscription } from 'rxjs';

import { ApiErrorResponseDto } from '../../dto/api-error-response.dto';
import { SearchResourceEntryDto } from '../../dto/search-resource-entry.dto';
import { SearchResourcesResponseDto } from '../../dto/search-resources-response.dto';
import { TPipe } from '../../pipes/t.pipe';
import { ClientSessionService } from '../../services/client-session.service';
import { I18nService } from '../../services/i18n.service';
import { StorageApiService } from '../../services/storage-api.service';

@Component({
  selector: 'app-search-page',
  imports: [CommonModule, TPipe],
  templateUrl: './search-page.component.html',
  styleUrl: './search-page.component.css'
})
export class SearchPageComponent implements OnInit, OnDestroy {
  private static readonly SEARCH_PAGE_LIMIT = 120;

  private routeSub: Subscription | null = null;
  private loadSub: Subscription | null = null;
  private ownerAvatarUrls = new Map<number, string>();
  private ownerAvatarFallbackIds = new Set<number>();
  private ownerAvatarAccessToken: string | null = null;

  query = '';
  entries: SearchResourceEntryDto[] = [];
  nextCursor: string | null = null;
  hasMore = false;
  loading = false;
  loadingMore = false;
  errorMessage = '';

  constructor(
    private readonly route: ActivatedRoute,
    private readonly router: Router,
    private readonly sessionService: ClientSessionService,
    private readonly i18nService: I18nService,
    private readonly storageApiService: StorageApiService,
    private readonly cdr: ChangeDetectorRef
  ) {}

  ngOnInit(): void {
    this.routeSub = this.route.queryParamMap.subscribe((params) => {
      const nextQuery = (params.get('q') ?? '').trim();
      this.resetAndLoad(nextQuery);
    });
  }

  ngOnDestroy(): void {
    this.routeSub?.unsubscribe();
    this.routeSub = null;
    this.loadSub?.unsubscribe();
    this.loadSub = null;
  }

  openResource(entry: SearchResourceEntryDto): void {
    if (entry.resourceType === 'folder') {
      this.openFolder(entry.resourceId, entry.sourceContext);
      return;
    }

    if (entry.sourceContext === 'shared') {
      this.router.navigate(['/app/storage/files', entry.resourceId], {
        queryParams: { source: 'shared' }
      });
      return;
    }

    this.router.navigate(['/app/storage/files', entry.resourceId]);
  }

  goToLocation(entry: SearchResourceEntryDto): void {
    const folderId = Number.isInteger(entry.navigateFolderId) && entry.navigateFolderId > 0
      ? entry.navigateFolderId
      : null;

    if (folderId === null) {
      this.openResource(entry);
      return;
    }

    this.openFolder(folderId, entry.sourceContext);
  }

  loadMore(): void {
    if (this.loading || this.loadingMore || !this.hasMore || !this.nextCursor) {
      return;
    }

    this.fetchResults(false);
  }

  ownerAvatarSrc(ownerUserId: number): string {
    if (!Number.isFinite(ownerUserId) || ownerUserId <= 0) {
      return 'profile.png';
    }

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
    this.ownerAvatarUrls.delete(ownerUserId);

    const target = event.target as HTMLImageElement | null;
    if (!target) {
      return;
    }

    target.onerror = null;
    target.src = 'profile.png';
    target.classList.add('default-avatar');
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

  sourceLabel(entry: SearchResourceEntryDto): string {
    return this.i18nService.t(
      entry.sourceContext === 'shared' ? 'search.source.shared' : 'search.source.storage'
    );
  }

  private resetAndLoad(nextQuery: string): void {
    this.loadSub?.unsubscribe();
    this.loadSub = null;

    this.query = nextQuery;
    this.entries = [];
    this.nextCursor = null;
    this.hasMore = false;
    this.errorMessage = '';
    this.loading = false;
    this.loadingMore = false;

    if (this.query.length === 0) {
      this.cdr.detectChanges();
      return;
    }

    this.fetchResults(true);
  }

  private fetchResults(reset: boolean): void {
    const accessToken = this.sessionService.readAccessToken();
    if (!accessToken) {
      this.redirectToLogin();
      return;
    }

    if (reset) {
      this.loading = true;
    } else {
      this.loadingMore = true;
    }
    this.errorMessage = '';

    this.loadSub?.unsubscribe();
    this.loadSub = this.storageApiService
      .searchResources(
        '',
        accessToken,
        this.query,
        SearchPageComponent.SEARCH_PAGE_LIMIT,
        reset ? null : this.nextCursor
      )
      .pipe(finalize(() => {
        this.loading = false;
        this.loadingMore = false;
        this.loadSub = null;
        this.cdr.detectChanges();
      }))
      .subscribe({
        next: (payload: SearchResourcesResponseDto) => {
          const incomingEntries = payload.entries;
          if (reset) {
            this.entries = incomingEntries;
          } else {
            this.entries = this.mergeEntries(this.entries, incomingEntries);
          }

          this.nextCursor = payload.nextCursor;
          this.hasMore = payload.hasMore;
          this.cdr.detectChanges();
        },
        error: (error: unknown) => {
          this.errorMessage = this.extractError(error, 'Failed to load search results');

          if (error instanceof HttpErrorResponse && error.status === 401) {
            this.redirectToLogin();
            return;
          }

          this.cdr.detectChanges();
        }
      });
  }

  private mergeEntries(
    currentEntries: SearchResourceEntryDto[],
    incomingEntries: SearchResourceEntryDto[]
  ): SearchResourceEntryDto[] {
    const seen = new Set(currentEntries.map((entry) => this.entryKey(entry)));
    const merged = [...currentEntries];

    for (const entry of incomingEntries) {
      const key = this.entryKey(entry);
      if (seen.has(key)) {
        continue;
      }

      seen.add(key);
      merged.push(entry);
    }

    return merged;
  }

  private entryKey(entry: SearchResourceEntryDto): string {
    return `${entry.sourceContext}:${entry.resourceType}:${entry.resourceId}`;
  }

  private openFolder(folderId: number, sourceContext: 'storage' | 'shared'): void {
    if (sourceContext === 'shared') {
      this.router.navigate(['/app/storage/folders', folderId], {
        queryParams: { source: 'shared' }
      });
      return;
    }

    this.router.navigate(['/app/storage/folders', folderId]);
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
