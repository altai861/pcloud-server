import { CommonModule } from '@angular/common';
import { HttpErrorResponse } from '@angular/common/http';
import { ChangeDetectorRef, Component, OnDestroy, OnInit } from '@angular/core';
import { Router } from '@angular/router';
import { debounceTime, distinctUntilChanged, finalize, Subscription } from 'rxjs';

import { ApiErrorResponseDto } from '../../dto/api-error-response.dto';
import { StorageEntryDto } from '../../dto/storage-entry.dto';
import { StorageListResponseDto } from '../../dto/storage-list-response.dto';
import { ClientSessionService } from '../../services/client-session.service';
import { RecentEntriesService } from '../../services/recent-entries.service';
import { StorageApiService } from '../../services/storage-api.service';
import { WorkspaceSearchService } from '../../services/workspace-search.service';

type ViewMode = 'list' | 'grid';

interface BreadcrumbItem {
  label: string;
  path: string;
}

@Component({
  selector: 'app-storage-home',
  imports: [CommonModule],
  templateUrl: './storage-home.component.html',
  styleUrl: './storage-home.component.css'
})
export class StorageHomeComponent implements OnInit, OnDestroy {
  private searchSub: Subscription | null = null;
  private searchTerm = '';

  currentPath = '/';
  parentPath: string | null = null;
  entries: StorageEntryDto[] = [];

  storageLoading = false;
  storageErrorMessage = '';
  viewMode: ViewMode = 'list';

  constructor(
    private readonly storageApiService: StorageApiService,
    private readonly sessionService: ClientSessionService,
    private readonly recentEntriesService: RecentEntriesService,
    private readonly searchService: WorkspaceSearchService,
    private readonly router: Router,
    private readonly cdr: ChangeDetectorRef
  ) {}

  ngOnInit(): void {
    this.searchSub = this.searchService.searchTerm$
      .pipe(debounceTime(260), distinctUntilChanged())
      .subscribe((term) => {
        this.searchTerm = term.trim();
        this.loadStorage(this.currentPath);
      });
  }

  ngOnDestroy(): void {
    this.searchSub?.unsubscribe();
    this.searchSub = null;
  }

  get breadcrumbs(): BreadcrumbItem[] {
    const result: BreadcrumbItem[] = [{ label: '/', path: '/' }];

    if (this.currentPath === '/') {
      return result;
    }

    const chunks = this.currentPath.split('/').filter((item) => item.length > 0);
    let pathAccumulator = '';

    for (const chunk of chunks) {
      pathAccumulator += `/${chunk}`;
      result.push({
        label: chunk,
        path: pathAccumulator
      });
    }

    return result;
  }

  openPath(path: string): void {
    this.loadStorage(path);
  }

  openEntry(entry: StorageEntryDto): void {
    this.recentEntriesService.recordOpened(entry);

    if (entry.entryType !== 'folder') {
      return;
    }

    this.loadStorage(entry.path);
  }

  setViewMode(mode: ViewMode): void {
    this.viewMode = mode;
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

  formatDate(unixMillis: number | null): string {
    if (unixMillis === null || !Number.isFinite(unixMillis)) {
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

  private loadStorage(path: string): void {
    const accessToken = this.sessionService.readAccessToken();

    if (!accessToken) {
      this.redirectToLogin();
      return;
    }

    this.storageLoading = true;
    this.storageErrorMessage = '';

    this.storageApiService
      .list('', accessToken, path, this.searchTerm)
      .pipe(finalize(() => {
        this.storageLoading = false;
        this.cdr.detectChanges();
      }))
      .subscribe({
        next: (body: StorageListResponseDto) => {
          this.currentPath = body.currentPath;
          this.parentPath = body.parentPath;
          this.entries = body.entries;
          this.cdr.detectChanges();
        },
        error: (error: unknown) => {
          this.storageErrorMessage = this.extractError(error, 'Failed to load storage list');

          if (error instanceof HttpErrorResponse && error.status === 401) {
            this.redirectToLogin();
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
