import { CommonModule } from '@angular/common';
import { HttpErrorResponse } from '@angular/common/http';
import { ChangeDetectorRef, Component, HostListener, OnDestroy, OnInit } from '@angular/core';
import { Router } from '@angular/router';
import { debounceTime, distinctUntilChanged, finalize, firstValueFrom, Subscription } from 'rxjs';

import { ApiErrorResponseDto } from '../../dto/api-error-response.dto';
import { StorageEntryDto } from '../../dto/storage-entry.dto';
import { StorageListResponseDto } from '../../dto/storage-list-response.dto';
import { TPipe } from '../../pipes/t.pipe';
import { ClientSessionService } from '../../services/client-session.service';
import { StorageApiService } from '../../services/storage-api.service';
import { StorageSidebarActionsService } from '../../services/storage-sidebar-actions.service';
import { WorkspaceSearchService } from '../../services/workspace-search.service';

@Component({
  selector: 'app-trash-page',
  imports: [CommonModule, TPipe],
  templateUrl: './trash-page.component.html',
  styleUrl: './trash-page.component.css'
})
export class TrashPageComponent implements OnInit, OnDestroy {
  private listSub: Subscription | null = null;
  private searchSub: Subscription | null = null;
  private searchTerm = '';

  entries: StorageEntryDto[] = [];
  trashLoading = false;
  trashErrorMessage = '';
  isEntryMenuOpen = false;
  isEntryMenuForMultiSelection = false;
  entryMenuX = 0;
  entryMenuY = 0;
  entryMenuTarget: StorageEntryDto | null = null;
  selectedEntryKeys = new Set<string>();
  isBulkDeleteInProgress = false;

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
        this.loadTrash();
      });

    this.loadTrash();
  }

  ngOnDestroy(): void {
    this.listSub?.unsubscribe();
    this.listSub = null;
    this.searchSub?.unsubscribe();
    this.searchSub = null;
  }

  get selectedEntriesCount(): number {
    let count = 0;

    for (const entry of this.entries) {
      if (this.isEntrySelected(entry)) {
        count += 1;
      }
    }

    return count;
  }

  isEntrySelected(entry: StorageEntryDto): boolean {
    return this.selectedEntryKeys.has(this.entrySelectionKey(entry));
  }

  onEntryRowClick(event: MouseEvent, entry: StorageEntryDto): void {
    if (this.isBulkDeleteInProgress || this.trashLoading) {
      return;
    }

    const additive = event.ctrlKey || event.metaKey;
    const key = this.entrySelectionKey(entry);

    if (additive) {
      if (this.selectedEntryKeys.has(key)) {
        this.selectedEntryKeys.delete(key);
      } else {
        this.selectedEntryKeys.add(key);
      }
    } else {
      this.selectedEntryKeys.clear();
      this.selectedEntryKeys.add(key);
    }

    this.cdr.detectChanges();
  }

  clearSelection(): void {
    if (this.selectedEntryKeys.size === 0) {
      return;
    }

    this.selectedEntryKeys.clear();
    this.cdr.detectChanges();
  }

  async onPermanentDeleteSelected(): Promise<void> {
    if (this.isBulkDeleteInProgress || this.trashLoading) {
      return;
    }

    const selectedEntries = this.entries.filter((entry) => this.isEntrySelected(entry));
    if (selectedEntries.length === 0) {
      this.trashErrorMessage = 'Select at least one item to delete permanently';
      this.cdr.detectChanges();
      return;
    }

    if (!window.confirm(`Permanently delete ${selectedEntries.length} selected item(s)? This cannot be undone.`)) {
      return;
    }

    const accessToken = this.sessionService.readAccessToken();
    if (!accessToken) {
      this.redirectToLogin();
      return;
    }

    this.closeEntryMenu();
    this.isBulkDeleteInProgress = true;
    this.trashLoading = true;
    this.trashErrorMessage = '';
    this.cdr.detectChanges();

    let failedCount = 0;
    let lastError = '';

    for (const entry of selectedEntries) {
      const request = entry.entryType === 'folder'
        ? this.storageApiService.permanentlyDeleteFolder('', accessToken, entry.path)
        : this.storageApiService.permanentlyDeleteFile('', accessToken, entry.path);

      try {
        await firstValueFrom(request);
      } catch (error: unknown) {
        failedCount += 1;

        if (error instanceof HttpErrorResponse && error.status === 401) {
          this.isBulkDeleteInProgress = false;
          this.trashLoading = false;
          this.redirectToLogin();
          return;
        }

        lastError = this.extractError(error, `Failed to permanently delete ${entry.entryType}`);
      }
    }

    this.isBulkDeleteInProgress = false;
    this.trashLoading = false;

    if (failedCount > 0) {
      const successCount = selectedEntries.length - failedCount;
      this.trashErrorMessage = successCount > 0
        ? `${successCount} item(s) permanently deleted, ${failedCount} failed. ${lastError}`.trim()
        : `${failedCount} item(s) failed to permanently delete. ${lastError}`.trim();
    }

    this.storageSidebarActions.notifyUsageChanged();
    this.loadTrash();
  }

  openEntryMenuFromButton(event: MouseEvent, entry: StorageEntryDto): void {
    event.preventDefault();
    event.stopPropagation();

    if (!this.isEntrySelected(entry)) {
      this.selectedEntryKeys.clear();
      this.selectedEntryKeys.add(this.entrySelectionKey(entry));
    }

    this.openEntryMenuAt(event.clientX, event.clientY, entry, this.selectedEntriesCount > 1);
  }

  openEntryContextMenu(event: MouseEvent, entry: StorageEntryDto): void {
    event.preventDefault();
    event.stopPropagation();

    if (!this.isEntrySelected(entry)) {
      this.selectedEntryKeys.clear();
      this.selectedEntryKeys.add(this.entrySelectionKey(entry));
    }

    this.openEntryMenuAt(event.clientX, event.clientY, entry, this.selectedEntriesCount > 1);
  }

  closeEntryMenu(): void {
    this.isEntryMenuOpen = false;
    this.isEntryMenuForMultiSelection = false;
    this.entryMenuTarget = null;
  }

  onRestoreEntry(event: MouseEvent): void {
    event.preventDefault();
    event.stopPropagation();

    const entry = this.entryMenuTarget;
    if (!entry) {
      return;
    }

    if (!window.confirm(`Restore "${entry.name}" from Trash?`)) {
      return;
    }

    const accessToken = this.sessionService.readAccessToken();
    if (!accessToken) {
      this.redirectToLogin();
      return;
    }

    this.closeEntryMenu();
    this.trashLoading = true;
    this.trashErrorMessage = '';

    const request = entry.entryType === 'folder'
      ? this.storageApiService.restoreFolder('', accessToken, entry.path)
      : this.storageApiService.restoreFile('', accessToken, entry.path);

    request
      .pipe(finalize(() => {
        this.trashLoading = false;
        this.cdr.detectChanges();
      }))
      .subscribe({
        next: () => {
          this.loadTrash();
        },
        error: (error: unknown) => {
          this.trashErrorMessage = this.extractError(error, `Failed to restore ${entry.entryType}`);

          if (error instanceof HttpErrorResponse && error.status === 401) {
            this.redirectToLogin();
          }

          this.cdr.detectChanges();
        }
      });
  }

  onPermanentDelete(event: MouseEvent): void {
    event.preventDefault();
    event.stopPropagation();

    if (this.isEntryMenuForMultiSelection) {
      void this.onPermanentDeleteSelected();
      return;
    }

    const entry = this.entryMenuTarget;
    if (!entry) {
      return;
    }

    const isFolder = entry.entryType === 'folder';
    const confirmMessage = isFolder
      ? `Permanently delete folder "${entry.name}" and all its contents? This cannot be undone.`
      : `Permanently delete file "${entry.name}"? This cannot be undone.`;

    if (!window.confirm(confirmMessage)) {
      return;
    }

    const accessToken = this.sessionService.readAccessToken();
    if (!accessToken) {
      this.redirectToLogin();
      return;
    }

    this.closeEntryMenu();
    this.trashLoading = true;
    this.trashErrorMessage = '';

    const request = isFolder
      ? this.storageApiService.permanentlyDeleteFolder('', accessToken, entry.path)
      : this.storageApiService.permanentlyDeleteFile('', accessToken, entry.path);

    request
      .pipe(finalize(() => {
        this.trashLoading = false;
        this.cdr.detectChanges();
      }))
      .subscribe({
        next: () => {
          this.storageSidebarActions.notifyUsageChanged();
          this.loadTrash();
        },
        error: (error: unknown) => {
          this.trashErrorMessage = this.extractError(error, `Failed to permanently delete ${entry.entryType}`);

          if (error instanceof HttpErrorResponse && error.status === 401) {
            this.redirectToLogin();
          }

          this.cdr.detectChanges();
        }
      });
  }

  @HostListener('document:click', ['$event'])
  onDocumentClick(event: MouseEvent): void {
    const target = event.target as HTMLElement | null;
    const clickedInsideRow = !!target?.closest('.list-row');
    const clickedInsideMenu = !!target?.closest('.entry-context-menu');
    const additive = event.ctrlKey || event.metaKey;

    if (this.isEntryMenuOpen) {
      this.closeEntryMenu();
      this.cdr.detectChanges();
    }

    if (!additive && !clickedInsideRow && !clickedInsideMenu) {
      this.clearSelection();
    }
  }

  @HostListener('document:keydown.escape')
  onEscapePressed(): void {
    if (!this.isEntryMenuOpen) {
      return;
    }

    this.closeEntryMenu();
    this.cdr.detectChanges();
  }

  @HostListener('document:keydown', ['$event'])
  onDocumentKeyDown(event: KeyboardEvent): void {
    if (!(event.ctrlKey || event.metaKey)) {
      return;
    }

    if (event.key.toLowerCase() !== 'a') {
      return;
    }

    if (this.isTypingTarget(event.target as HTMLElement | null)) {
      return;
    }

    if (this.entries.length === 0 || this.trashLoading || this.isBulkDeleteInProgress) {
      return;
    }

    event.preventDefault();

    this.selectedEntryKeys.clear();
    for (const entry of this.entries) {
      this.selectedEntryKeys.add(this.entrySelectionKey(entry));
    }

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

  private loadTrash(): void {
    const accessToken = this.sessionService.readAccessToken();
    if (!accessToken) {
      this.redirectToLogin();
      return;
    }

    this.listSub?.unsubscribe();
    this.trashLoading = true;
    this.trashErrorMessage = '';

    this.listSub = this.storageApiService
      .listTrash('', accessToken, this.searchTerm)
      .pipe(finalize(() => {
        this.listSub = null;
        this.trashLoading = false;
        this.cdr.detectChanges();
      }))
      .subscribe({
        next: (payload: StorageListResponseDto) => {
          this.entries = payload.entries;
          this.pruneSelectedEntries();
          this.cdr.detectChanges();
        },
        error: (error: unknown) => {
          this.trashErrorMessage = this.extractError(error, 'Failed to load trash list');

          if (error instanceof HttpErrorResponse && error.status === 401) {
            this.redirectToLogin();
          }

          this.cdr.detectChanges();
        }
      });
  }

  private openEntryMenuAt(clientX: number, clientY: number, entry: StorageEntryDto, multiSelection: boolean): void {
    const menuWidth = 190;
    const menuHeight = 98;
    const viewportPadding = 8;

    this.entryMenuX = Math.max(
      viewportPadding,
      Math.min(clientX, window.innerWidth - menuWidth - viewportPadding)
    );
    this.entryMenuY = Math.max(
      viewportPadding,
      Math.min(clientY, window.innerHeight - menuHeight - viewportPadding)
    );
    this.isEntryMenuForMultiSelection = multiSelection;
    this.entryMenuTarget = entry;
    this.isEntryMenuOpen = true;
    this.cdr.detectChanges();
  }

  private entrySelectionKey(entry: StorageEntryDto): string {
    return `${entry.entryType}:${entry.id}`;
  }

  private pruneSelectedEntries(): void {
    if (this.selectedEntryKeys.size === 0) {
      return;
    }

    const visibleKeys = new Set(this.entries.map((entry) => this.entrySelectionKey(entry)));
    for (const key of Array.from(this.selectedEntryKeys)) {
      if (!visibleKeys.has(key)) {
        this.selectedEntryKeys.delete(key);
      }
    }
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

  private isTypingTarget(target: HTMLElement | null): boolean {
    if (!target) {
      return false;
    }

    const tag = target.tagName.toLowerCase();
    if (tag === 'input' || tag === 'textarea' || tag === 'select') {
      return true;
    }

    if (target.isContentEditable) {
      return true;
    }

    return target.closest('[contenteditable=\"true\"]') !== null;
  }
}
