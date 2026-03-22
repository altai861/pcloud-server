import { CommonModule } from '@angular/common';
import { HttpErrorResponse } from '@angular/common/http';
import { ChangeDetectorRef, Component, HostListener, OnDestroy, OnInit } from '@angular/core';
import { Router } from '@angular/router';
import { debounceTime, distinctUntilChanged, finalize, Subscription } from 'rxjs';

import { ApiErrorResponseDto } from '../../dto/api-error-response.dto';
import { StorageEntryDto } from '../../dto/storage-entry.dto';
import { StorageListResponseDto } from '../../dto/storage-list-response.dto';
import { ClientSessionService } from '../../services/client-session.service';
import { StorageApiService } from '../../services/storage-api.service';
import { StorageSidebarActionsService } from '../../services/storage-sidebar-actions.service';
import { WorkspaceSearchService } from '../../services/workspace-search.service';

@Component({
  selector: 'app-starred-page',
  imports: [CommonModule],
  templateUrl: './starred-page.component.html',
  styleUrl: './starred-page.component.css'
})
export class StarredPageComponent implements OnInit, OnDestroy {
  private listSub: Subscription | null = null;
  private searchSub: Subscription | null = null;
  private searchTerm = '';

  entries: StorageEntryDto[] = [];
  loading = false;
  errorMessage = '';
  isEntryMenuOpen = false;
  entryMenuX = 0;
  entryMenuY = 0;
  entryMenuTarget: StorageEntryDto | null = null;

  constructor(
    private readonly storageApiService: StorageApiService,
    private readonly sessionService: ClientSessionService,
    private readonly storageSidebarActions: StorageSidebarActionsService,
    private readonly searchService: WorkspaceSearchService,
    private readonly router: Router,
    private readonly cdr: ChangeDetectorRef
  ) {}

  ngOnInit(): void {
    this.searchSub = this.searchService.searchTerm$
      .pipe(debounceTime(260), distinctUntilChanged())
      .subscribe((term) => {
        this.searchTerm = term.trim();
        this.loadStarred();
      });

    this.loadStarred();
  }

  ngOnDestroy(): void {
    this.listSub?.unsubscribe();
    this.listSub = null;
    this.searchSub?.unsubscribe();
    this.searchSub = null;
  }

  openEntryMenuFromButton(event: MouseEvent, entry: StorageEntryDto): void {
    event.preventDefault();
    event.stopPropagation();
    this.openEntryMenuAt(event.clientX, event.clientY, entry);
  }

  openEntryContextMenu(event: MouseEvent, entry: StorageEntryDto): void {
    event.preventDefault();
    event.stopPropagation();
    this.openEntryMenuAt(event.clientX, event.clientY, entry);
  }

  closeEntryMenu(): void {
    this.isEntryMenuOpen = false;
    this.entryMenuTarget = null;
  }

  openEntryInStorage(event: MouseEvent): void {
    event.preventDefault();
    event.stopPropagation();

    const entry = this.entryMenuTarget;
    if (!entry) {
      return;
    }

    const targetPath = this.resolveStorageOpenPath(entry);
    this.closeEntryMenu();

    this.storageSidebarActions.queue({
      type: 'open-path',
      path: targetPath
    });

    this.router.navigate(['/app/storage']);
  }

  onUnstarClick(event: MouseEvent, entry: StorageEntryDto): void {
    event.preventDefault();
    event.stopPropagation();
    this.updateStarredState(entry, false);
  }

  onUnstarFromMenu(event: MouseEvent): void {
    event.preventDefault();
    event.stopPropagation();

    const entry = this.entryMenuTarget;
    if (!entry) {
      return;
    }

    this.closeEntryMenu();
    this.updateStarredState(entry, false);
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
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit'
    }).format(date);
  }

  private loadStarred(): void {
    const accessToken = this.sessionService.readAccessToken();
    if (!accessToken) {
      this.redirectToLogin();
      return;
    }

    this.listSub?.unsubscribe();
    this.loading = true;
    this.errorMessage = '';

    this.listSub = this.storageApiService
      .listStarred('', accessToken, this.searchTerm)
      .pipe(finalize(() => {
        this.listSub = null;
        this.loading = false;
        this.cdr.detectChanges();
      }))
      .subscribe({
        next: (payload: StorageListResponseDto) => {
          this.entries = payload.entries;
          this.cdr.detectChanges();
        },
        error: (error: unknown) => {
          this.errorMessage = this.extractError(error, 'Failed to load starred items');

          if (error instanceof HttpErrorResponse && error.status === 401) {
            this.redirectToLogin();
          }

          this.cdr.detectChanges();
        }
      });
  }

  private openEntryMenuAt(clientX: number, clientY: number, entry: StorageEntryDto): void {
    const menuWidth = 196;
    const menuHeight = 92;
    const viewportPadding = 8;

    this.entryMenuX = Math.max(
      viewportPadding,
      Math.min(clientX, window.innerWidth - menuWidth - viewportPadding)
    );
    this.entryMenuY = Math.max(
      viewportPadding,
      Math.min(clientY, window.innerHeight - menuHeight - viewportPadding)
    );
    this.entryMenuTarget = entry;
    this.isEntryMenuOpen = true;
    this.cdr.detectChanges();
  }

  private updateStarredState(entry: StorageEntryDto, starred: boolean): void {
    const accessToken = this.sessionService.readAccessToken();
    if (!accessToken) {
      this.redirectToLogin();
      return;
    }

    const previousValue = entry.isStarred;
    entry.isStarred = starred;
    this.cdr.detectChanges();

    this.storageApiService
      .setStarred('', accessToken, entry.path, entry.entryType, starred)
      .subscribe({
        next: (response) => {
          entry.isStarred = response.entry.isStarred;

          if (!response.entry.isStarred) {
            this.entries = this.entries.filter(
              (current) =>
                !(current.path === entry.path && current.entryType === entry.entryType)
            );
          }

          this.cdr.detectChanges();
        },
        error: (error: unknown) => {
          entry.isStarred = previousValue;
          this.errorMessage = this.extractError(error, 'Failed to update star status');

          if (error instanceof HttpErrorResponse && error.status === 401) {
            this.redirectToLogin();
          }

          this.cdr.detectChanges();
        }
      });
  }

  private resolveStorageOpenPath(entry: StorageEntryDto): string {
    if (entry.path === '/') {
      return '/';
    }

    const chunks = entry.path.split('/').filter((segment) => segment.length > 0);
    if (chunks.length <= 1) {
      return '/';
    }

    return `/${chunks.slice(0, -1).join('/')}`;
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
