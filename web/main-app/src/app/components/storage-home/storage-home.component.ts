import { CommonModule } from '@angular/common';
import { HttpErrorResponse, HttpEventType } from '@angular/common/http';
import { ChangeDetectorRef, Component, HostListener, OnDestroy, OnInit, ViewRef } from '@angular/core';
import { Router } from '@angular/router';
import { debounceTime, distinctUntilChanged, finalize, Subscription } from 'rxjs';

import { ApiErrorResponseDto } from '../../dto/api-error-response.dto';
import { StorageEntryDto } from '../../dto/storage-entry.dto';
import { StorageFolderMetadataDto } from '../../dto/storage-folder-metadata.dto';
import { StorageListResponseDto } from '../../dto/storage-list-response.dto';
import { ClientSessionService } from '../../services/client-session.service';
import { ProfileImageService } from '../../services/profile-image.service';
import { RecentEntriesService } from '../../services/recent-entries.service';
import { StorageApiService } from '../../services/storage-api.service';
import { StorageSidebarAction, StorageSidebarActionsService } from '../../services/storage-sidebar-actions.service';
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
  private actionSub: Subscription | null = null;
  private searchSub: Subscription | null = null;
  private listLoadSub: Subscription | null = null;
  private profileImageSub: Subscription | null = null;
  private listRequestId = 0;
  private searchTerm = '';

  currentPath = '/';
  parentPath: string | null = null;
  entries: StorageEntryDto[] = [];

  storageLoading = false;
  storageErrorMessage = '';
  viewMode: ViewMode = 'list';
  isUploadInProgress = false;
  uploadProgressPercent: number | null = null;
  uploadFileName = '';
  isFolderMetadataOpen = false;
  isFolderMetadataLoading = false;
  folderMetadataErrorMessage = '';
  folderMetadata: StorageFolderMetadataDto | null = null;
  isEntryMenuOpen = false;
  entryMenuX = 0;
  entryMenuY = 0;
  entryMenuTarget: StorageEntryDto | null = null;
  isRenameModalOpen = false;
  renameTarget: StorageEntryDto | null = null;
  renameValue = '';
  renameErrorMessage = '';
  renameSubmitting = false;
  ownerProfileImageSrc: string | null = null;

  constructor(
    private readonly storageApiService: StorageApiService,
    private readonly sessionService: ClientSessionService,
    private readonly profileImageService: ProfileImageService,
    private readonly recentEntriesService: RecentEntriesService,
    private readonly storageSidebarActions: StorageSidebarActionsService,
    private readonly searchService: WorkspaceSearchService,
    private readonly router: Router,
    private readonly cdr: ChangeDetectorRef
  ) { }

  ngOnInit(): void {
    this.profileImageSub = this.profileImageService.profileImageSrc$.subscribe((src) => {
      this.ownerProfileImageSrc = src;
      this.triggerUiRefresh();
    });

    this.searchSub = this.searchService.searchTerm$
      .pipe(debounceTime(260), distinctUntilChanged())
      .subscribe((term) => {
        this.searchTerm = term.trim();
        this.loadStorage(this.currentPath);
      });

    this.actionSub = this.storageSidebarActions.actions$.subscribe((action) => {
      this.handleSidebarAction(action);
    });

    for (const action of this.storageSidebarActions.consumeQueued()) {
      this.handleSidebarAction(action);
    }
  }

  ngOnDestroy(): void {
    this.searchSub?.unsubscribe();
    this.searchSub = null;
    this.actionSub?.unsubscribe();
    this.actionSub = null;
    this.listLoadSub?.unsubscribe();
    this.listLoadSub = null;
    this.profileImageSub?.unsubscribe();
    this.profileImageSub = null;
    this.listRequestId += 1;
  }

  get breadcrumbs(): BreadcrumbItem[] {
    const result: BreadcrumbItem[] = [{ label: 'My Storage', path: '/' }];

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

  onStarClick(event: MouseEvent, entry: StorageEntryDto): void {
    event.preventDefault();
    event.stopPropagation();

    const accessToken = this.sessionService.readAccessToken();
    if (!accessToken) {
      this.redirectToLogin();
      return;
    }

    const nextValue = !entry.isStarred;
    const previousValue = entry.isStarred;
    entry.isStarred = nextValue;
    this.triggerUiRefresh();

    this.storageApiService
      .setStarred('', accessToken, entry.path, entry.entryType, nextValue)
      .subscribe({
        next: (response) => {
          entry.isStarred = response.entry.isStarred;
          this.triggerUiRefresh();
        },
        error: (error: unknown) => {
          entry.isStarred = previousValue;
          this.storageErrorMessage = this.extractError(error, `Failed to update ${entry.entryType} star status`);

          if (error instanceof HttpErrorResponse && error.status === 401) {
            this.redirectToLogin();
          }

          this.triggerUiRefresh();
        }
      });
  }

  setViewMode(mode: ViewMode): void {
    this.viewMode = mode;
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

  onDownloadEntryClick(event: MouseEvent): void {
    event.preventDefault();
    event.stopPropagation();

    const selectedEntry = this.entryMenuTarget;
    if (!selectedEntry || selectedEntry.entryType !== 'file') {
      return;
    }

    this.closeEntryMenu();
    this.downloadEntry(selectedEntry);
  }

  onRenameEntryClick(event: MouseEvent): void {
    event.preventDefault();
    event.stopPropagation();

    const selectedEntry = this.entryMenuTarget;
    if (!selectedEntry) {
      return;
    }

    this.closeEntryMenu();
    this.renameTarget = selectedEntry;
    this.renameValue = selectedEntry.name;
    this.renameErrorMessage = '';
    this.renameSubmitting = false;
    this.isRenameModalOpen = true;
    this.cdr.detectChanges();
  }

  onDeleteEntryClick(event: MouseEvent): void {
    event.preventDefault();
    event.stopPropagation();

    const selectedEntry = this.entryMenuTarget;
    if (!selectedEntry) {
      return;
    }

    const isFolder = selectedEntry.entryType === 'folder';
    const confirmMessage = isFolder
      ? `Move folder "${selectedEntry.name}" and all its contents to Trash?`
      : `Move file "${selectedEntry.name}" to Trash?`;

    if (!window.confirm(confirmMessage)) {
      return;
    }

    const accessToken = this.sessionService.readAccessToken();
    if (!accessToken) {
      this.redirectToLogin();
      return;
    }

    this.closeEntryMenu();
    this.storageLoading = true;
    this.storageErrorMessage = '';

    const request = isFolder
      ? this.storageApiService.deleteFolder('', accessToken, selectedEntry.path)
      : this.storageApiService.deleteFile('', accessToken, selectedEntry.path);

    request
      .pipe(finalize(() => {
        this.storageLoading = false;
        this.cdr.detectChanges();
      }))
      .subscribe({
        next: () => {
          this.storageSidebarActions.notifyUsageChanged();
          this.loadStorage(this.currentPath);
        },
        error: (error: unknown) => {
          this.storageErrorMessage = this.extractError(error, `Failed to move ${selectedEntry.entryType} to trash`);

          if (error instanceof HttpErrorResponse && error.status === 401) {
            this.redirectToLogin();
          }

          this.cdr.detectChanges();
        }
      });
  }

  closeRenameModal(): void {
    this.isRenameModalOpen = false;
    this.renameTarget = null;
    this.renameValue = '';
    this.renameErrorMessage = '';
    this.renameSubmitting = false;
  }

  submitRename(): void {
    const target = this.renameTarget;
    if (!target) {
      return;
    }

    const nextName = this.renameValue.trim();
    if (nextName.length === 0) {
      this.renameErrorMessage = 'Name cannot be empty';
      this.cdr.detectChanges();
      return;
    }

    if (nextName === target.name) {
      this.closeRenameModal();
      this.cdr.detectChanges();
      return;
    }

    const accessToken = this.sessionService.readAccessToken();
    if (!accessToken) {
      this.redirectToLogin();
      return;
    }

    this.renameSubmitting = true;
    this.renameErrorMessage = '';

    const request = target.entryType === 'folder'
      ? this.storageApiService.renameFolder('', accessToken, target.path, nextName)
      : this.storageApiService.renameFile('', accessToken, target.path, nextName);

    request
      .pipe(finalize(() => {
        this.renameSubmitting = false;
        this.cdr.detectChanges();
      }))
      .subscribe({
        next: () => {
          this.closeRenameModal();
          this.loadStorage(this.currentPath);
        },
        error: (error: unknown) => {
          this.renameErrorMessage = this.extractError(error, `Failed to rename ${target.entryType}`);

          if (error instanceof HttpErrorResponse && error.status === 401) {
            this.redirectToLogin();
          }

          this.cdr.detectChanges();
        }
      });
  }

  get isEntryDownloadDisabled(): boolean {
    return !this.entryMenuTarget || this.entryMenuTarget.entryType !== 'file';
  }

  openFolderMetadata(): void {
    const accessToken = this.sessionService.readAccessToken();
    if (!accessToken) {
      this.redirectToLogin();
      return;
    }

    this.isFolderMetadataOpen = true;
    this.isFolderMetadataLoading = true;
    this.folderMetadataErrorMessage = '';
    this.folderMetadata = null;

    this.storageApiService
      .folderMetadata('', accessToken, this.currentPath)
      .pipe(finalize(() => {
        this.isFolderMetadataLoading = false;
        this.cdr.detectChanges();
      }))
      .subscribe({
        next: (payload: StorageFolderMetadataDto) => {
          this.folderMetadata = payload;
          this.cdr.detectChanges();
        },
        error: (error: unknown) => {
          this.folderMetadataErrorMessage = this.extractError(error, 'Failed to load folder metadata');

          if (error instanceof HttpErrorResponse && error.status === 401) {
            this.redirectToLogin();
          }

          this.cdr.detectChanges();
        }
      });
  }

  closeFolderMetadata(): void {
    this.isFolderMetadataOpen = false;
    this.folderMetadataErrorMessage = '';
    this.folderMetadata = null;
    this.isFolderMetadataLoading = false;
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

  formatDateTime(unixMillis: number | null): string {
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
    if (this.isRenameModalOpen) {
      this.closeRenameModal();
      this.cdr.detectChanges();
      return;
    }

    if (!this.isEntryMenuOpen) {
      return;
    }

    this.closeEntryMenu();
    this.cdr.detectChanges();
  }

  private loadStorage(path: string): void {
    const accessToken = this.sessionService.readAccessToken();

    if (!accessToken) {
      this.redirectToLogin();
      return;
    }

    this.listLoadSub?.unsubscribe();
    const requestId = ++this.listRequestId;

    this.storageLoading = true;
    this.storageErrorMessage = '';

    this.listLoadSub = this.storageApiService
      .list('', accessToken, path, this.searchTerm)
      .pipe(finalize(() => {
        if (requestId !== this.listRequestId) {
          return;
        }

        this.listLoadSub = null;
        this.storageLoading = false;
        this.triggerUiRefresh();
      }))
      .subscribe({
        next: (body: StorageListResponseDto) => {
          if (requestId !== this.listRequestId) {
            return;
          }

          this.currentPath = body.currentPath;
          this.parentPath = body.parentPath;
          this.entries = body.entries;
          this.triggerUiRefresh();
        },
        error: (error: unknown) => {
          if (requestId !== this.listRequestId) {
            return;
          }

          this.storageErrorMessage = this.extractError(error, 'Failed to load storage list');

          if (error instanceof HttpErrorResponse && error.status === 401) {
            this.redirectToLogin();
          }

          this.triggerUiRefresh();
        }
      });
  }

  private handleSidebarAction(action: StorageSidebarAction): void {
    if (action.type === 'create-folder') {
      this.openCreateFolderPrompt();
      return;
    }

    if (action.type === 'upload-file') {
      this.uploadFile(action.file);
      return;
    }

    this.loadStorage(action.path);
  }

  private openCreateFolderPrompt(): void {
    const folderName = window.prompt('Enter folder name');
    if (folderName === null) {
      return;
    }

    const name = folderName.trim();
    if (name.length === 0) {
      this.storageErrorMessage = 'Folder name cannot be empty';
      this.cdr.detectChanges();
      return;
    }

    const accessToken = this.sessionService.readAccessToken();
    if (!accessToken) {
      this.redirectToLogin();
      return;
    }

    this.storageLoading = true;
    this.storageErrorMessage = '';

    this.storageApiService
      .createFolder('', accessToken, this.currentPath, name)
      .pipe(finalize(() => {
        this.storageLoading = false;
        this.cdr.detectChanges();
      }))
      .subscribe({
        next: () => {
          this.loadStorage(this.currentPath);
        },
        error: (error: unknown) => {
          this.storageErrorMessage = this.extractError(error, 'Failed to create folder');

          if (error instanceof HttpErrorResponse && error.status === 401) {
            this.redirectToLogin();
          }

          this.cdr.detectChanges();
        }
      });
  }

  private uploadFile(file: File): void {
    const accessToken = this.sessionService.readAccessToken();
    if (!accessToken) {
      this.redirectToLogin();
      return;
    }

    this.isUploadInProgress = true;
    this.uploadProgressPercent = 0;
    this.uploadFileName = file.name;
    this.storageLoading = true;
    this.storageErrorMessage = '';

    this.storageApiService
      .uploadFile('', accessToken, this.currentPath, file)
      .pipe(finalize(() => {
        this.isUploadInProgress = false;
        this.uploadProgressPercent = null;
        this.uploadFileName = '';
        this.storageLoading = false;
        this.cdr.detectChanges();
      }))
      .subscribe({
        next: (event) => {
          if (event.type === HttpEventType.UploadProgress) {
            if (event.total && event.total > 0) {
              this.uploadProgressPercent = Math.min(
                100,
                Math.round((event.loaded / event.total) * 100)
              );
            } else {
              this.uploadProgressPercent = null;
            }
            this.cdr.detectChanges();
            return;
          }

          if (event.type === HttpEventType.Response) {
            this.uploadProgressPercent = 100;
            this.storageSidebarActions.notifyUsageChanged();
            this.loadStorage(this.currentPath);
          }
        },
        error: (error: unknown) => {
          this.storageErrorMessage = this.extractError(error, 'Failed to upload file');

          if (error instanceof HttpErrorResponse && error.status === 401) {
            this.redirectToLogin();
          }

          this.cdr.detectChanges();
        }
      });
  }

  private openEntryMenuAt(clientX: number, clientY: number, entry: StorageEntryDto): void {
    const menuWidth = 176;
    const menuHeight = 132;
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

  private downloadEntry(entry: StorageEntryDto): void {
    if (entry.entryType !== 'file') {
      this.storageErrorMessage = 'Folder download is not available yet';
      this.cdr.detectChanges();
      return;
    }

    const accessToken = this.sessionService.readAccessToken();
    if (!accessToken) {
      this.redirectToLogin();
      return;
    }

    const downloadUrl = this.storageApiService.buildFileDownloadUrl(
      '',
      accessToken,
      entry.path
    );

    const anchor = document.createElement('a');
    anchor.href = downloadUrl;
    anchor.target = '_blank';
    anchor.rel = 'noopener noreferrer';
    document.body.appendChild(anchor);
    anchor.click();
    anchor.remove();
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

  private triggerUiRefresh(): void {
    const view = this.cdr as ViewRef;
    if (view.destroyed) {
      return;
    }

    this.cdr.detectChanges();
  }
}
