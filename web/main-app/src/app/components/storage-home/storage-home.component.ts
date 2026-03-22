import { CommonModule } from '@angular/common';
import { HttpErrorResponse, HttpEventType, HttpResponse } from '@angular/common/http';
import { ChangeDetectorRef, Component, HostListener, OnDestroy, OnInit, ViewRef } from '@angular/core';
import { ActivatedRoute, Router } from '@angular/router';
import { debounceTime, distinctUntilChanged, finalize, firstValueFrom, Subscription } from 'rxjs';

import { ApiErrorResponseDto } from '../../dto/api-error-response.dto';
import { ShareableUserDto } from '../../dto/shareable-user.dto';
import { SharedPermissionTargetDto } from '../../dto/shared-permission-target.dto';
import { StorageEntryDto } from '../../dto/storage-entry.dto';
import { StorageFolderMetadataDto } from '../../dto/storage-folder-metadata.dto';
import { StorageListResponseDto } from '../../dto/storage-list-response.dto';
import { ClientSessionService } from '../../services/client-session.service';
import { I18nService } from '../../services/i18n.service';
import { ProfileImageService } from '../../services/profile-image.service';
import { RecentEntriesService } from '../../services/recent-entries.service';
import { StorageApiService } from '../../services/storage-api.service';
import { StorageSidebarAction, StorageSidebarActionsService } from '../../services/storage-sidebar-actions.service';
import { WorkspaceSearchService } from '../../services/workspace-search.service';
import { TPipe } from '../../pipes/t.pipe';

type ViewMode = 'list' | 'grid';

interface BreadcrumbItem {
  label: string;
  path: string;
  canOpen: boolean;
}

type FilterMenuKind = 'type' | 'people' | 'modified' | null;
type NameSortOrder = 'asc' | 'desc';
type SortField = 'name' | 'modified' | 'size';
type ModifiedFilterValue = 'any' | 'last-day' | 'last-week' | 'last-month' | 'last-year';
type TypeFilterValue =
  | 'all'
  | 'folder'
  | 'file-any'
  | 'image'
  | 'document'
  | 'pdf'
  | 'spreadsheet'
  | 'presentation'
  | 'archive'
  | 'audio'
  | 'video'
  | 'code';

interface FilterOption<T extends string> {
  value: T;
  labelKey: string;
}

interface PeopleFilterOption {
  userId: number;
  username: string;
}

const IMAGE_EXTENSIONS = new Set([
  'jpg', 'jpeg', 'png', 'gif', 'webp', 'bmp', 'svg', 'heic', 'heif', 'tif', 'tiff', 'ico', 'avif'
]);
const DOCUMENT_EXTENSIONS = new Set([
  'txt', 'md', 'rtf', 'doc', 'docx', 'odt', 'pages', 'epub', 'pdf'
]);
const PDF_EXTENSIONS = new Set(['pdf']);
const SPREADSHEET_EXTENSIONS = new Set(['xls', 'xlsx', 'ods', 'csv', 'tsv', 'numbers']);
const PRESENTATION_EXTENSIONS = new Set(['ppt', 'pptx', 'odp', 'key']);
const ARCHIVE_EXTENSIONS = new Set(['zip', 'rar', '7z', 'tar', 'gz', 'bz2', 'xz', 'tgz']);
const AUDIO_EXTENSIONS = new Set(['mp3', 'wav', 'flac', 'ogg', 'aac', 'm4a', 'wma']);
const VIDEO_EXTENSIONS = new Set(['mp4', 'mov', 'mkv', 'webm', 'avi', 'wmv', 'flv', 'm4v']);
const CODE_EXTENSIONS = new Set([
  'rs', 'ts', 'js', 'jsx', 'tsx', 'html', 'css', 'scss', 'json', 'yaml', 'yml', 'xml', 'toml', 'go', 'java', 'py', 'c', 'cpp', 'h', 'hpp', 'php', 'sh'
]);

@Component({
  selector: 'app-storage-home',
  imports: [CommonModule, TPipe],
  templateUrl: './storage-home.component.html',
  styleUrl: './storage-home.component.css'
})
export class StorageHomeComponent implements OnInit, OnDestroy {
  private static readonly STORAGE_PAGE_LIMIT = 200;

  private actionSub: Subscription | null = null;
  private searchSub: Subscription | null = null;
  private routeSub: Subscription | null = null;
  private listLoadSub: Subscription | null = null;
  private profileImageSub: Subscription | null = null;
  private listRequestId = 0;
  private searchTerm = '';
  private navigationSource: 'storage' | 'shared' = 'storage';
  private ownerAvatarUrls = new Map<number, string>();
  private ownerAvatarFallbackIds = new Set<number>();
  private ownerAvatarAccessToken: string | null = null;

  currentPath = '/';
  currentFolderId: number | null = null;
  parentPath: string | null = null;
  currentPrivilege: 'owner' | 'editor' | 'viewer' = 'owner';
  entries: StorageEntryDto[] = [];
  nextEntriesCursor: string | null = null;
  hasMoreEntries = false;

  storageLoading = false;
  isLoadingMore = false;
  storageErrorMessage = '';
  viewMode: ViewMode = 'list';
  isUploadInProgress = false;
  uploadProgressPercent: number | null = null;
  uploadFileName = '';
  uploadBatchTotal = 0;
  uploadBatchIndex = 0;
  isFolderMetadataOpen = false;
  isFolderMetadataLoading = false;
  folderMetadataErrorMessage = '';
  folderMetadata: StorageFolderMetadataDto | null = null;
  isEntryMenuOpen = false;
  isEntryMenuForMultiSelection = false;
  entryMenuX = 0;
  entryMenuY = 0;
  entryMenuTarget: StorageEntryDto | null = null;
  isRenameModalOpen = false;
  renameTarget: StorageEntryDto | null = null;
  renameValue = '';
  renameErrorMessage = '';
  renameSubmitting = false;
  isMoveModalOpen = false;
  moveSubmitting = false;
  moveErrorMessage = '';
  moveTargets: StorageEntryDto[] = [];
  moveBrowserLoading = false;
  moveBrowserPath = '/';
  moveBrowserFolderId: number | null = null;
  moveBrowserParentFolderId: number | null = null;
  moveBrowserFolders: StorageEntryDto[] = [];
  moveDestinationFolderId: number | null = null;
  moveDestinationPath = '/';
  ownerProfileImageSrc: string | null = null;
  isShareModalOpen = false;
  shareModalEntry: StorageEntryDto | null = null;
  shareModalErrorMessage = '';
  sharePermissionsLoading = false;
  sharePermissions: SharedPermissionTargetDto[] = [];
  shareUserSearchTerm = '';
  shareUsersLoading = false;
  shareUsers: ShareableUserDto[] = [];
  selectedShareUserId: number | null = null;
  selectedSharePrivilege: 'viewer' | 'editor' = 'viewer';
  shareMutationLoading = false;
  selectedEntryKeys = new Set<string>();
  isBulkTrashInProgress = false;
  isBatchDownloadInProgress = false;

  activeFilterMenu: FilterMenuKind = null;
  typeFilter: TypeFilterValue = 'all';
  modifiedFilter: ModifiedFilterValue = 'any';
  activeSortField: SortField = 'name';
  nameSortOrder: NameSortOrder = 'asc';
  modifiedSortOrder: NameSortOrder = 'desc';
  sizeSortOrder: NameSortOrder = 'desc';
  peopleSearchTerm = '';
  selectedPeopleUserIds = new Set<number>();

  readonly typeFilterOptions: FilterOption<TypeFilterValue>[] = [
    { value: 'all', labelKey: 'storage.filter.allResources' },
    { value: 'folder', labelKey: 'storage.filter.foldersOnly' },
    { value: 'file-any', labelKey: 'storage.filter.anyFile' },
    { value: 'image', labelKey: 'storage.filter.images' },
    { value: 'document', labelKey: 'storage.filter.documents' },
    { value: 'pdf', labelKey: 'storage.filter.pdfFiles' },
    { value: 'spreadsheet', labelKey: 'storage.filter.spreadsheets' },
    { value: 'presentation', labelKey: 'storage.filter.presentations' },
    { value: 'archive', labelKey: 'storage.filter.archives' },
    { value: 'audio', labelKey: 'storage.filter.audio' },
    { value: 'video', labelKey: 'storage.filter.video' },
    { value: 'code', labelKey: 'storage.filter.codeFiles' }
  ];

  readonly modifiedFilterOptions: FilterOption<ModifiedFilterValue>[] = [
    { value: 'any', labelKey: 'common.filter.anyTime' },
    { value: 'last-day', labelKey: 'common.filter.lastDay' },
    { value: 'last-week', labelKey: 'common.filter.lastWeek' },
    { value: 'last-month', labelKey: 'common.filter.lastMonth' },
    { value: 'last-year', labelKey: 'common.filter.lastYear' }
  ];

  constructor(
    private readonly storageApiService: StorageApiService,
    private readonly sessionService: ClientSessionService,
    private readonly i18nService: I18nService,
    private readonly profileImageService: ProfileImageService,
    private readonly recentEntriesService: RecentEntriesService,
    private readonly storageSidebarActions: StorageSidebarActionsService,
    private readonly searchService: WorkspaceSearchService,
    private readonly route: ActivatedRoute,
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
        this.reloadCurrentFolder();
      });

    this.routeSub = this.route.paramMap.subscribe((params) => {
      const source = this.route.snapshot.queryParamMap.get('source');
      this.navigationSource = source === 'shared' ? 'shared' : 'storage';
      this.resetFiltersForNavigation();

      const rawFolderId = params.get('folderId') ?? 'root';

      if (rawFolderId === 'root') {
        this.loadStorageByPath('/', true);
        return;
      }

      const folderId = Number(rawFolderId);
      if (!Number.isInteger(folderId) || folderId <= 0) {
        this.storageErrorMessage = 'Invalid folder route';
        this.entries = [];
        this.selectedEntryKeys.clear();
        this.storageLoading = false;
        this.triggerUiRefresh();
        return;
      }

      this.loadStorageByFolderId(folderId);
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
    this.routeSub?.unsubscribe();
    this.routeSub = null;
    this.actionSub?.unsubscribe();
    this.actionSub = null;
    this.listLoadSub?.unsubscribe();
    this.listLoadSub = null;
    this.profileImageSub?.unsubscribe();
    this.profileImageSub = null;
    this.listRequestId += 1;
  }

  get breadcrumbs(): BreadcrumbItem[] {
    const isSharedContext = this.isSharedContext;
    const result: BreadcrumbItem[] = [{
      label: isSharedContext ? 'Shared with me' : 'My Storage',
      path: '/',
      canOpen: true
    }];

    if (this.currentPath === '/') {
      return result;
    }

    const chunks = this.currentPath.split('/').filter((item) => item.length > 0);
    let pathAccumulator = '';

    for (const chunk of chunks) {
      pathAccumulator += `/${chunk}`;
      result.push({
        label: chunk,
        path: pathAccumulator,
        canOpen: !isSharedContext
      });
    }

    return result;
  }

  openPath(path: string): void {
    if (this.isSharedContext) {
      if (path === '/') {
        this.router.navigate(['/app/shared']);
      }
      return;
    }

    this.loadStorageByPath(path, true);
  }

  openEntry(entry: StorageEntryDto): void {
    this.recentEntriesService.recordOpened(entry);

    if (entry.entryType !== 'folder') {
      return;
    }

    this.navigateToFolderRoute(entry.id);
  }

  ownerAvatarSrc(ownerUserId: number | null): string {
    if (ownerUserId === null || !Number.isFinite(ownerUserId)) {
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

  onOwnerAvatarError(event: Event, ownerUserId: number | null): void {
    if (ownerUserId !== null && Number.isFinite(ownerUserId)) {
      this.ownerAvatarFallbackIds.add(ownerUserId);
    }

    const image = event.target as HTMLImageElement | null;
    if (image) {
      image.src = 'profile.png';
    }
  }

  isDefaultOwnerAvatar(ownerUserId: number | null): boolean {
    if (ownerUserId === null || !Number.isFinite(ownerUserId)) {
      return true;
    }

    return this.ownerAvatarFallbackIds.has(ownerUserId);
  }

  onEntryDoubleClick(entry: StorageEntryDto): void {
    if (entry.entryType === 'file') {
      if (this.isSharedContext) {
        this.router.navigate(['/app/storage/files', entry.id], {
          queryParams: { source: 'shared' }
        });
      } else {
        this.router.navigate(['/app/storage/files', entry.id]);
      }
      return;
    }

    this.navigateToFolderRoute(entry.id);
  }

  onStarClick(event: MouseEvent, entry: StorageEntryDto): void {
    event.preventDefault();
    event.stopPropagation();

    if (this.currentPrivilege !== 'owner') {
      this.storageErrorMessage = 'Starred status can only be changed from your own storage';
      this.triggerUiRefresh();
      return;
    }

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

  get displayEntries(): StorageEntryDto[] {
    let filtered = this.entries.filter((entry) => this.matchesTypeFilter(entry));

    if (this.selectedPeopleUserIds.size > 0) {
      filtered = filtered.filter(
        (entry) =>
          this.selectedPeopleUserIds.has(entry.ownerUserId) ||
          (entry.createdByUserId !== null && this.selectedPeopleUserIds.has(entry.createdByUserId))
      );
    }

    const modifiedThreshold = this.modifiedSinceThreshold();
    if (modifiedThreshold !== null) {
      filtered = filtered.filter((entry) => {
        if (entry.modifiedAtUnixMs === null || !Number.isFinite(entry.modifiedAtUnixMs)) {
          return false;
        }

        return entry.modifiedAtUnixMs >= modifiedThreshold;
      });
    }

    return [...filtered].sort((left, right) => {
      if (this.activeSortField === 'modified') {
        const byModified = this.compareNullableNumbers(
          left.modifiedAtUnixMs,
          right.modifiedAtUnixMs,
          this.modifiedSortOrder
        );
        if (byModified !== 0) {
          return byModified;
        }
      } else if (this.activeSortField === 'size') {
        const bySize = this.compareNullableNumbers(
          left.sizeBytes,
          right.sizeBytes,
          this.sizeSortOrder
        );
        if (bySize !== 0) {
          return bySize;
        }
      } else {
        const byName = this.compareNames(left.name, right.name, this.nameSortOrder);
        if (byName !== 0) {
          return byName;
        }
      }

      const byName = this.compareNames(left.name, right.name, 'asc');
      if (byName !== 0) {
        return byName;
      }

      if (left.entryType !== right.entryType) {
        return left.entryType === 'folder' ? -1 : 1;
      }

      return left.id - right.id;
    });
  }

  get peopleFilterOptions(): PeopleFilterOption[] {
    const optionsById = new Map<number, string>();

    for (const entry of this.entries) {
      optionsById.set(entry.ownerUserId, entry.ownerUsername);

      if (entry.createdByUserId !== null && Number.isFinite(entry.createdByUserId)) {
        optionsById.set(entry.createdByUserId, entry.createdByUsername);
      }
    }

    const normalizedSearch = this.peopleSearchTerm.trim().toLowerCase();

    return Array.from(optionsById.entries())
      .map(([userId, username]) => ({ userId, username }))
      .filter((option) => {
        if (normalizedSearch.length === 0) {
          return true;
        }

        return option.username.toLowerCase().includes(normalizedSearch);
      })
      .sort((left, right) =>
        left.username.localeCompare(right.username, undefined, { sensitivity: 'base' })
      );
  }

  get typeFilterLabel(): string {
    const key = this.typeFilterOptions.find((option) => option.value === this.typeFilter)?.labelKey
      ?? 'storage.filter.type';
    return this.i18nService.t(key);
  }

  get modifiedFilterLabel(): string {
    const key = this.modifiedFilterOptions.find((option) => option.value === this.modifiedFilter)?.labelKey
      ?? 'storage.filter.modified';
    return this.i18nService.t(key);
  }

  get sortLabel(): string {
    return this.nameSortOrder === 'asc' ? 'A-Z' : 'Z-A';
  }

  get modifiedSortLabel(): string {
    return this.modifiedSortOrder === 'asc' ? 'Old-New' : 'New-Old';
  }

  get sizeSortLabel(): string {
    return this.sizeSortOrder === 'asc' ? 'Small-Large' : 'Large-Small';
  }

  get hasActiveFilters(): boolean {
    return (
      this.typeFilter !== 'all' ||
      this.modifiedFilter !== 'any' ||
      this.selectedPeopleUserIds.size > 0
    );
  }

  toggleFilterMenu(kind: Exclude<FilterMenuKind, null>, event: MouseEvent): void {
    event.preventDefault();
    event.stopPropagation();
    this.activeFilterMenu = this.activeFilterMenu === kind ? null : kind;
    this.triggerUiRefresh();
  }

  closeFilterMenu(): void {
    this.activeFilterMenu = null;
  }

  setTypeFilter(value: TypeFilterValue): void {
    this.typeFilter = value;
    this.pruneSelectedEntries();
    this.triggerUiRefresh();
  }

  setModifiedFilter(value: ModifiedFilterValue): void {
    this.modifiedFilter = value;
    this.pruneSelectedEntries();
    this.triggerUiRefresh();
  }

  isPeopleSelected(userId: number): boolean {
    return this.selectedPeopleUserIds.has(userId);
  }

  togglePeopleSelection(userId: number): void {
    if (this.selectedPeopleUserIds.has(userId)) {
      this.selectedPeopleUserIds.delete(userId);
    } else {
      this.selectedPeopleUserIds.add(userId);
    }

    this.pruneSelectedEntries();
    this.triggerUiRefresh();
  }

  clearPeopleSelection(): void {
    if (this.selectedPeopleUserIds.size === 0) {
      return;
    }

    this.selectedPeopleUserIds.clear();
    this.pruneSelectedEntries();
    this.triggerUiRefresh();
  }

  clearAllFilters(): void {
    this.typeFilter = 'all';
    this.modifiedFilter = 'any';
    this.selectedPeopleUserIds.clear();
    this.peopleSearchTerm = '';
    this.activeFilterMenu = null;
    this.pruneSelectedEntries();
    this.triggerUiRefresh();
  }

  toggleSortOrder(): void {
    this.activeSortField = 'name';
    this.nameSortOrder = this.nameSortOrder === 'asc' ? 'desc' : 'asc';
    this.triggerUiRefresh();
  }

  toggleModifiedSortOrder(): void {
    this.activeSortField = 'modified';
    this.modifiedSortOrder = this.modifiedSortOrder === 'asc' ? 'desc' : 'asc';
    this.triggerUiRefresh();
  }

  toggleSizeSortOrder(): void {
    this.activeSortField = 'size';
    this.sizeSortOrder = this.sizeSortOrder === 'asc' ? 'desc' : 'asc';
    this.triggerUiRefresh();
  }

  get canEditCurrentFolder(): boolean {
    return this.currentPrivilege === 'owner' || this.currentPrivilege === 'editor';
  }

  get canMutateExistingEntries(): boolean {
    return this.currentPrivilege === 'owner';
  }

  get canRenameExistingEntries(): boolean {
    return this.currentPrivilege === 'owner' || this.currentPrivilege === 'editor';
  }

  get selectedEntriesCount(): number {
    let count = 0;

    for (const entry of this.displayEntries) {
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
    if (this.isBulkTrashInProgress || this.storageLoading) {
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

    this.triggerUiRefresh();
  }

  clearSelection(): void {
    if (this.selectedEntryKeys.size === 0) {
      return;
    }

    this.selectedEntryKeys.clear();
    this.triggerUiRefresh();
  }

  async moveSelectedToTrash(): Promise<void> {
    if (!this.canMutateExistingEntries) {
      this.storageErrorMessage = 'Only the owner can delete existing items in this folder';
      this.triggerUiRefresh();
      return;
    }

    if (this.isBulkTrashInProgress || this.storageLoading) {
      return;
    }

    const selectedEntries = this.displayEntries.filter((entry) => this.isEntrySelected(entry));
    if (selectedEntries.length === 0) {
      this.storageErrorMessage = 'Select at least one item to move to Trash';
      this.triggerUiRefresh();
      return;
    }

    if (!window.confirm(`Move ${selectedEntries.length} selected item(s) to Trash?`)) {
      return;
    }

    const accessToken = this.sessionService.readAccessToken();
    if (!accessToken) {
      this.redirectToLogin();
      return;
    }

    this.isBulkTrashInProgress = true;
    this.storageLoading = true;
    this.storageErrorMessage = '';
    this.closeEntryMenu();
    this.triggerUiRefresh();

    let failedCount = 0;
    let lastError = '';

    for (const entry of selectedEntries) {
      const request = entry.entryType === 'folder'
        ? this.storageApiService.deleteFolder('', accessToken, entry.path)
        : this.storageApiService.deleteFile('', accessToken, entry.path);

      try {
        await firstValueFrom(request);
      } catch (error: unknown) {
        failedCount += 1;

        if (error instanceof HttpErrorResponse && error.status === 401) {
          this.isBulkTrashInProgress = false;
          this.storageLoading = false;
          this.redirectToLogin();
          return;
        }

        lastError = this.extractError(error, `Failed to move ${entry.entryType} to trash`);
      }
    }

    this.isBulkTrashInProgress = false;
    this.storageLoading = false;

    if (failedCount > 0) {
      const successCount = selectedEntries.length - failedCount;
      this.storageErrorMessage = successCount > 0
        ? `${successCount} item(s) moved to Trash, ${failedCount} failed. ${lastError}`.trim()
        : `${failedCount} item(s) failed to move to Trash. ${lastError}`.trim();
    }

    this.storageSidebarActions.notifyUsageChanged();
    this.reloadCurrentFolder();
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

  onShareEntryClick(event: MouseEvent): void {
    event.preventDefault();
    event.stopPropagation();

    if (this.currentPrivilege !== 'owner') {
      return;
    }

    const selectedEntry = this.entryMenuTarget;
    if (!selectedEntry) {
      return;
    }

    this.closeEntryMenu();
    this.openShareModal(selectedEntry);
  }

  onDownloadEntryClick(event: MouseEvent): void {
    event.preventDefault();
    event.stopPropagation();

    if (this.isBatchDownloadInProgress) {
      return;
    }

    const selectedEntries = this.displayEntries.filter((entry) => this.isEntrySelected(entry));
    if (this.isEntryMenuForMultiSelection) {
      if (selectedEntries.length === 0) {
        return;
      }

      this.closeEntryMenu();
      this.downloadSelectedAsArchive(selectedEntries);
      return;
    }

    const selectedEntry = this.entryMenuTarget;
    if (!selectedEntry) {
      return;
    }

    this.closeEntryMenu();
    if (selectedEntry.entryType === 'file') {
      this.downloadEntry(selectedEntry);
      return;
    }

    this.downloadSelectedAsArchive([selectedEntry]);
  }

  onRenameEntryClick(event: MouseEvent): void {
    event.preventDefault();
    event.stopPropagation();

    if (!this.canRenameExistingEntries) {
      this.storageErrorMessage = 'You need editor permission to rename items in this folder';
      this.cdr.detectChanges();
      return;
    }

    if (this.isEntryMenuForMultiSelection) {
      return;
    }

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

  onMoveEntryClick(event: MouseEvent): void {
    event.preventDefault();
    event.stopPropagation();

    if (!this.canMutateExistingEntries) {
      this.storageErrorMessage = this.i18nService.t('storage.moveErrorOwnerOnly');
      this.cdr.detectChanges();
      return;
    }

    let targets: StorageEntryDto[] = [];
    if (this.isEntryMenuForMultiSelection) {
      targets = this.displayEntries.filter((entry) => this.isEntrySelected(entry));
    } else if (this.entryMenuTarget) {
      targets = [this.entryMenuTarget];
    }

    if (targets.length === 0) {
      this.storageErrorMessage = this.i18nService.t('storage.moveErrorSelectAtLeastOne');
      this.cdr.detectChanges();
      return;
    }

    this.closeEntryMenu();
    this.openMoveModal(targets);
  }

  onDeleteEntryClick(event: MouseEvent): void {
    event.preventDefault();
    event.stopPropagation();

    if (this.isEntryMenuForMultiSelection) {
      void this.moveSelectedToTrash();
      return;
    }

    if (!this.canMutateExistingEntries) {
      this.storageErrorMessage = 'Only the owner can delete existing items in this folder';
      this.cdr.detectChanges();
      return;
    }

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
          this.reloadCurrentFolder();
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
    if (!this.canRenameExistingEntries) {
      this.renameErrorMessage = 'You need editor permission to rename items in this folder';
      this.cdr.detectChanges();
      return;
    }

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
      ? this.storageApiService.renameFolder('', accessToken, target.path, nextName, target.id)
      : this.storageApiService.renameFile('', accessToken, target.path, nextName, target.id);

    request
      .pipe(finalize(() => {
        this.renameSubmitting = false;
        this.cdr.detectChanges();
      }))
      .subscribe({
        next: () => {
          this.closeRenameModal();
          this.reloadCurrentFolder();
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

  get moveTargetsCount(): number {
    return this.moveTargets.length;
  }

  get moveDestinationWarning(): string | null {
    if (!this.isMoveModalOpen || this.moveDestinationFolderId === null) {
      return null;
    }

    for (const target of this.moveTargets) {
      if (target.entryType !== 'folder') {
        continue;
      }

      if (target.id === this.moveDestinationFolderId) {
        return this.i18nService.t('storage.moveWarnSelf');
      }

      if (this.isDescendantPath(this.moveDestinationPath, target.path)) {
        return this.i18nService.t('storage.moveWarnChild');
      }
    }

    return null;
  }

  get canSubmitMove(): boolean {
    if (!this.isMoveModalOpen || this.moveSubmitting || this.moveBrowserLoading) {
      return false;
    }

    if (!this.canMutateExistingEntries || this.moveTargets.length === 0) {
      return false;
    }

    if (this.moveDestinationFolderId === null) {
      return false;
    }

    return this.moveDestinationWarning === null;
  }

  openMoveModal(targets: StorageEntryDto[]): void {
    this.isMoveModalOpen = true;
    this.moveSubmitting = false;
    this.moveErrorMessage = '';
    this.moveTargets = [...targets];
    this.moveBrowserLoading = false;
    this.moveBrowserPath = '/';
    this.moveBrowserFolderId = null;
    this.moveBrowserParentFolderId = null;
    this.moveBrowserFolders = [];
    this.moveDestinationFolderId = null;
    this.moveDestinationPath = '/';
    this.cdr.detectChanges();

    this.loadMoveBrowserPath('/', null);
  }

  closeMoveModal(): void {
    this.isMoveModalOpen = false;
    this.moveSubmitting = false;
    this.moveErrorMessage = '';
    this.moveTargets = [];
    this.moveBrowserLoading = false;
    this.moveBrowserPath = '/';
    this.moveBrowserFolderId = null;
    this.moveBrowserParentFolderId = null;
    this.moveBrowserFolders = [];
    this.moveDestinationFolderId = null;
    this.moveDestinationPath = '/';
  }

  moveBrowserNavigateRoot(): void {
    if (this.moveBrowserLoading) {
      return;
    }

    this.loadMoveBrowserPath('/', null);
  }

  moveBrowserNavigateUp(): void {
    if (this.moveBrowserLoading || this.moveBrowserParentFolderId === null) {
      return;
    }

    this.loadMoveBrowserPath('', this.moveBrowserParentFolderId);
  }

  openMoveBrowserFolder(folder: StorageEntryDto): void {
    if (this.moveBrowserLoading || folder.entryType !== 'folder') {
      return;
    }

    this.loadMoveBrowserPath('', folder.id);
  }

  selectMoveDestination(folderId: number | null, path: string): void {
    if (folderId === null) {
      return;
    }

    this.moveDestinationFolderId = folderId;
    this.moveDestinationPath = path;
    this.moveErrorMessage = '';
    this.cdr.detectChanges();
  }

  submitMove(): void {
    if (!this.canMutateExistingEntries) {
      this.moveErrorMessage = this.i18nService.t('storage.moveErrorOwnerOnly');
      this.cdr.detectChanges();
      return;
    }

    const destinationFolderId = this.moveDestinationFolderId;
    if (destinationFolderId === null) {
      this.moveErrorMessage = this.i18nService.t('storage.moveErrorSelectDestination');
      this.cdr.detectChanges();
      return;
    }

    const warning = this.moveDestinationWarning;
    if (warning) {
      this.moveErrorMessage = warning;
      this.cdr.detectChanges();
      return;
    }

    const accessToken = this.sessionService.readAccessToken();
    if (!accessToken) {
      this.redirectToLogin();
      return;
    }

    this.moveSubmitting = true;
    this.moveErrorMessage = '';
    this.cdr.detectChanges();

    this.storageApiService
      .moveResources(
        '',
        accessToken,
        destinationFolderId,
        this.moveTargets.map((target) => ({
          entryType: target.entryType,
          resourceId: target.id
        }))
      )
      .pipe(finalize(() => {
        this.moveSubmitting = false;
        this.cdr.detectChanges();
      }))
      .subscribe({
        next: () => {
          this.closeMoveModal();
          this.selectedEntryKeys.clear();
          this.reloadCurrentFolder();
          this.cdr.detectChanges();
        },
        error: (error: unknown) => {
          this.moveErrorMessage = this.extractError(error, this.i18nService.t('storage.moveErrorSubmit'));

          if (error instanceof HttpErrorResponse && error.status === 401) {
            this.redirectToLogin();
            return;
          }

          this.cdr.detectChanges();
        }
      });
  }

  get isEntryDownloadDisabled(): boolean {
    if (this.isBatchDownloadInProgress || this.storageLoading || this.isBulkTrashInProgress) {
      return true;
    }

    if (this.isEntryMenuForMultiSelection) {
      return this.selectedEntriesCount === 0;
    }

    return !this.entryMenuTarget;
  }

  get entryDownloadLabel(): string {
    if (this.isBatchDownloadInProgress) {
      return 'Preparing archive...';
    }

    if (this.isEntryMenuForMultiSelection) {
      return 'Download selected';
    }

    if (this.entryMenuTarget?.entryType === 'folder') {
      return 'Download folder';
    }

    return 'Download file';
  }

  get isEntryEditDisabled(): boolean {
    if (this.isEntryMenuForMultiSelection) {
      return !this.canMutateExistingEntries || this.selectedEntriesCount === 0;
    }

    return !this.entryMenuTarget || !this.canMutateExistingEntries;
  }

  get isEntryMoveDisabled(): boolean {
    if (this.isEntryMenuForMultiSelection) {
      return !this.canMutateExistingEntries || this.selectedEntriesCount === 0;
    }

    return !this.entryMenuTarget || !this.canMutateExistingEntries;
  }

  get isEntryRenameDisabled(): boolean {
    if (this.isEntryMenuForMultiSelection) {
      return true;
    }

    return !this.entryMenuTarget || !this.canRenameExistingEntries;
  }

  get isEntryShareDisabled(): boolean {
    if (this.isEntryMenuForMultiSelection) {
      return true;
    }

    return !this.entryMenuTarget || this.currentPrivilege !== 'owner';
  }

  closeShareModal(): void {
    this.isShareModalOpen = false;
    this.shareModalEntry = null;
    this.shareModalErrorMessage = '';
    this.sharePermissionsLoading = false;
    this.sharePermissions = [];
    this.shareUserSearchTerm = '';
    this.shareUsersLoading = false;
    this.shareUsers = [];
    this.selectedShareUserId = null;
    this.selectedSharePrivilege = 'viewer';
    this.shareMutationLoading = false;
  }

  searchShareableUsers(): void {
    const entry = this.shareModalEntry;
    const accessToken = this.sessionService.readAccessToken();
    if (!entry || !accessToken) {
      return;
    }

    this.shareUsersLoading = true;
    this.shareModalErrorMessage = '';

    this.storageApiService
      .searchShareableUsers('', accessToken, this.shareUserSearchTerm)
      .pipe(finalize(() => {
        this.shareUsersLoading = false;
        this.cdr.detectChanges();
      }))
      .subscribe({
        next: (response) => {
          this.shareUsers = response.users;
          this.cdr.detectChanges();
        },
        error: (error: unknown) => {
          this.shareModalErrorMessage = this.extractError(error, 'Failed to search users');
          this.cdr.detectChanges();
        }
      });
  }

  selectShareUser(user: ShareableUserDto): void {
    this.selectedShareUserId = user.userId;
  }

  grantSharePermission(): void {
    const entry = this.shareModalEntry;
    const accessToken = this.sessionService.readAccessToken();
    if (!entry || !accessToken || this.selectedShareUserId === null) {
      return;
    }

    this.shareMutationLoading = true;
    this.shareModalErrorMessage = '';

    this.storageApiService
      .upsertSharePermission(
        '',
        accessToken,
        entry.entryType,
        entry.id,
        this.selectedShareUserId,
        this.selectedSharePrivilege
      )
      .pipe(finalize(() => {
        this.shareMutationLoading = false;
        this.cdr.detectChanges();
      }))
      .subscribe({
        next: () => {
          this.loadSharePermissions(entry);
        },
        error: (error: unknown) => {
          this.shareModalErrorMessage = this.extractError(error, 'Failed to share resource');
          this.cdr.detectChanges();
        }
      });
  }

  updateSharePermission(target: SharedPermissionTargetDto, privilegeType: 'viewer' | 'editor'): void {
    const entry = this.shareModalEntry;
    const accessToken = this.sessionService.readAccessToken();
    if (!entry || !accessToken) {
      return;
    }

    this.shareMutationLoading = true;
    this.shareModalErrorMessage = '';

    this.storageApiService
      .upsertSharePermission('', accessToken, entry.entryType, entry.id, target.userId, privilegeType)
      .pipe(finalize(() => {
        this.shareMutationLoading = false;
        this.cdr.detectChanges();
      }))
      .subscribe({
        next: () => {
          this.loadSharePermissions(entry);
        },
        error: (error: unknown) => {
          this.shareModalErrorMessage = this.extractError(error, 'Failed to update permission');
          this.cdr.detectChanges();
        }
      });
  }

  removeShareTarget(target: SharedPermissionTargetDto): void {
    const entry = this.shareModalEntry;
    const accessToken = this.sessionService.readAccessToken();
    if (!entry || !accessToken) {
      return;
    }

    this.shareMutationLoading = true;
    this.shareModalErrorMessage = '';

    this.storageApiService
      .removeSharePermission('', accessToken, entry.entryType, entry.id, target.userId)
      .pipe(finalize(() => {
        this.shareMutationLoading = false;
        this.cdr.detectChanges();
      }))
      .subscribe({
        next: () => {
          this.loadSharePermissions(entry);
        },
        error: (error: unknown) => {
          this.shareModalErrorMessage = this.extractError(error, 'Failed to remove permission');
          this.cdr.detectChanges();
        }
      });
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
      .folderMetadata('', accessToken, this.currentPath, this.currentFolderId)
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

  @HostListener('document:click', ['$event'])
  onDocumentClick(event: MouseEvent): void {
    const target = event.target as HTMLElement | null;
    const clickedInsideRow = !!target?.closest('.list-row');
    const clickedInsideMenu = !!target?.closest('.entry-context-menu');
    const clickedInsideFilters = !!target?.closest('.quick-filters');
    const additive = event.ctrlKey || event.metaKey;

    if (this.isEntryMenuOpen) {
      this.closeEntryMenu();
      this.cdr.detectChanges();
    }

    if (this.activeFilterMenu !== null && !clickedInsideFilters) {
      this.closeFilterMenu();
      this.cdr.detectChanges();
    }

    if (!additive && !clickedInsideRow && !clickedInsideMenu && !clickedInsideFilters) {
      this.clearSelection();
    }
  }

  @HostListener('document:keydown.escape')
  onEscapePressed(): void {
    if (this.isShareModalOpen) {
      this.closeShareModal();
      this.cdr.detectChanges();
      return;
    }

    if (this.isMoveModalOpen) {
      this.closeMoveModal();
      this.cdr.detectChanges();
      return;
    }

    if (this.isRenameModalOpen) {
      this.closeRenameModal();
      this.cdr.detectChanges();
      return;
    }

    if (this.activeFilterMenu !== null) {
      this.closeFilterMenu();
      this.cdr.detectChanges();
      return;
    }

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

    if (this.displayEntries.length === 0 || this.storageLoading || this.isBulkTrashInProgress) {
      return;
    }

    event.preventDefault();

    this.selectedEntryKeys.clear();
    for (const entry of this.displayEntries) {
      this.selectedEntryKeys.add(this.entrySelectionKey(entry));
    }

    this.triggerUiRefresh();
  }

  private loadStorageByFolderId(folderId: number): void {
    this.loadStorageInternal('', folderId, false, false);
  }

  private loadStorageByPath(path: string, syncRoute: boolean): void {
    this.loadStorageInternal(path, null, syncRoute, false);
  }

  private loadMoveBrowserPath(path: string, folderId: number | null): void {
    const accessToken = this.sessionService.readAccessToken();
    if (!accessToken) {
      this.redirectToLogin();
      return;
    }

    this.moveBrowserLoading = true;
    this.moveErrorMessage = '';
    this.cdr.detectChanges();

    this.storageApiService
      .list('', accessToken, path, '', folderId, 500, null)
      .pipe(finalize(() => {
        this.moveBrowserLoading = false;
        this.cdr.detectChanges();
      }))
      .subscribe({
        next: (body: StorageListResponseDto) => {
          this.moveBrowserPath = body.currentPath;
          this.moveBrowserFolderId = body.currentFolderId;
          this.moveBrowserParentFolderId = body.parentFolderId;
          this.moveBrowserFolders = body.entries
            .filter((entry) => entry.entryType === 'folder')
            .sort((left, right) =>
              left.name.localeCompare(right.name, undefined, {
                sensitivity: 'base',
                numeric: true
              })
            );

          if (this.moveDestinationFolderId === null && body.currentFolderId !== null) {
            this.moveDestinationFolderId = body.currentFolderId;
            this.moveDestinationPath = body.currentPath;
          }

          this.cdr.detectChanges();
        },
        error: (error: unknown) => {
          this.moveErrorMessage = this.extractError(error, this.i18nService.t('storage.moveErrorLoadFolders'));

          if (error instanceof HttpErrorResponse && error.status === 401) {
            this.redirectToLogin();
            return;
          }

          this.cdr.detectChanges();
        }
      });
  }

  loadMoreEntries(): void {
    if (this.storageLoading || this.isLoadingMore || !this.hasMoreEntries || !this.nextEntriesCursor) {
      return;
    }

    if (this.currentFolderId !== null) {
      this.loadStorageInternal('', this.currentFolderId, false, true);
      return;
    }

    this.loadStorageInternal(this.currentPath, null, false, true);
  }

  private loadStorageInternal(path: string, folderId: number | null, syncRoute: boolean, append: boolean): void {
    const accessToken = this.sessionService.readAccessToken();

    if (!accessToken) {
      this.redirectToLogin();
      return;
    }

    this.listLoadSub?.unsubscribe();
    const requestId = ++this.listRequestId;

    const requestCursor = append ? this.nextEntriesCursor : null;

    if (append) {
      this.isLoadingMore = true;
    } else {
      this.storageLoading = true;
      this.storageErrorMessage = '';
      this.nextEntriesCursor = null;
      this.hasMoreEntries = false;
    }

    this.listLoadSub = this.storageApiService
      .list(
        '',
        accessToken,
        path,
        this.searchTerm,
        folderId,
        StorageHomeComponent.STORAGE_PAGE_LIMIT,
        requestCursor
      )
      .pipe(finalize(() => {
        if (requestId !== this.listRequestId) {
          return;
        }

        this.listLoadSub = null;
        if (append) {
          this.isLoadingMore = false;
        } else {
          this.storageLoading = false;
        }
        this.triggerUiRefresh();
      }))
      .subscribe({
        next: (body: StorageListResponseDto) => {
          if (requestId !== this.listRequestId) {
            return;
          }

          this.currentPath = body.currentPath;
          this.currentFolderId = body.currentFolderId;
          this.parentPath = body.parentPath;
          this.currentPrivilege = body.currentPrivilege;
          this.entries = append ? [...this.entries, ...body.entries] : body.entries;
          this.pruneSelectedEntries();
          this.nextEntriesCursor = body.nextCursor;
          this.hasMoreEntries = body.hasMore;
          this.ownerAvatarAccessToken = accessToken;

          if (syncRoute && !append && body.currentFolderId !== null) {
            this.navigateToFolderRoute(body.currentFolderId);
          }

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

    if (action.type === 'upload-files') {
      this.uploadFiles(action.files);
      return;
    }

    if (action.type === 'upload-file') {
      this.uploadFiles([action.file]);
      return;
    }

    this.loadStorageByPath(action.path, true);
  }

  private openCreateFolderPrompt(): void {
    if (!this.canEditCurrentFolder) {
      this.storageErrorMessage = 'You need editor permission to create folders in this location';
      this.cdr.detectChanges();
      return;
    }

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
      .createFolder('', accessToken, this.currentPath, this.currentFolderId, name)
      .pipe(finalize(() => {
        this.storageLoading = false;
        this.cdr.detectChanges();
      }))
      .subscribe({
        next: () => {
          this.reloadCurrentFolder();
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

  private uploadFiles(files: File[]): void {
    if (files.length === 0) {
      return;
    }

    if (!this.canEditCurrentFolder) {
      this.storageErrorMessage = 'You need editor permission to upload files in this location';
      this.cdr.detectChanges();
      return;
    }

    const accessToken = this.sessionService.readAccessToken();
    if (!accessToken) {
      this.redirectToLogin();
      return;
    }

    const queue = [...files];

    this.isUploadInProgress = true;
    this.uploadBatchTotal = queue.length;
    this.uploadBatchIndex = 0;
    this.uploadProgressPercent = null;
    this.uploadFileName = '';
    this.storageLoading = true;
    this.storageErrorMessage = '';
    this.cdr.detectChanges();

    this.uploadNextFile(queue, accessToken);
  }

  private uploadNextFile(queue: File[], accessToken: string): void {
    if (queue.length === 0) {
      this.finishUploadBatch(true);
      return;
    }

    const file = queue.shift() as File;
    this.uploadBatchIndex += 1;
    this.uploadProgressPercent = 0;
    this.uploadFileName = this.uploadBatchTotal > 1
      ? `${file.name} (${this.uploadBatchIndex}/${this.uploadBatchTotal})`
      : file.name;
    this.cdr.detectChanges();

    this.storageApiService
      .uploadFile('', accessToken, this.currentPath, this.currentFolderId, file)
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
            this.cdr.detectChanges();
            this.uploadNextFile(queue, accessToken);
          }
        },
        error: (error: unknown) => {
          this.storageErrorMessage = this.extractError(error, 'Failed to upload file');

          if (error instanceof HttpErrorResponse && error.status === 401) {
            this.redirectToLogin();
            return;
          }

          this.finishUploadBatch(true);
        }
      });
  }

  private finishUploadBatch(reloadStorageList: boolean): void {
    this.isUploadInProgress = false;
    this.uploadProgressPercent = null;
    this.uploadFileName = '';
    this.uploadBatchTotal = 0;
    this.uploadBatchIndex = 0;
    this.storageLoading = false;

    if (reloadStorageList) {
      this.storageSidebarActions.notifyUsageChanged();
      this.reloadCurrentFolder();
      return;
    }

    this.cdr.detectChanges();
  }

  private openEntryMenuAt(clientX: number, clientY: number, entry: StorageEntryDto, multiSelection: boolean): void {
    const menuWidth = 176;
    const menuHeight = multiSelection ? 196 : 228;
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

    const visibleKeys = new Set(this.displayEntries.map((entry) => this.entrySelectionKey(entry)));
    for (const key of Array.from(this.selectedEntryKeys)) {
      if (!visibleKeys.has(key)) {
        this.selectedEntryKeys.delete(key);
      }
    }
  }

  private openShareModal(entry: StorageEntryDto): void {
    this.isShareModalOpen = true;
    this.shareModalEntry = entry;
    this.shareModalErrorMessage = '';
    this.sharePermissions = [];
    this.shareUsers = [];
    this.selectedShareUserId = null;
    this.selectedSharePrivilege = 'viewer';
    this.shareUserSearchTerm = '';
    this.loadSharePermissions(entry);
    this.cdr.detectChanges();
  }

  private loadSharePermissions(entry: StorageEntryDto): void {
    const accessToken = this.sessionService.readAccessToken();
    if (!accessToken) {
      this.redirectToLogin();
      return;
    }

    this.sharePermissionsLoading = true;
    this.shareModalErrorMessage = '';

    this.storageApiService
      .listSharePermissions('', accessToken, entry.entryType, entry.id)
      .pipe(finalize(() => {
        this.sharePermissionsLoading = false;
        this.cdr.detectChanges();
      }))
      .subscribe({
        next: (response) => {
          this.sharePermissions = response.entries;
          this.cdr.detectChanges();
        },
        error: (error: unknown) => {
          this.shareModalErrorMessage = this.extractError(error, 'Failed to load sharing permissions');
          this.cdr.detectChanges();
        }
      });
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
      entry.path,
      entry.id
    );

    const anchor = document.createElement('a');
    anchor.href = downloadUrl;
    anchor.target = '_blank';
    anchor.rel = 'noopener noreferrer';
    document.body.appendChild(anchor);
    anchor.click();
    anchor.remove();
  }

  private downloadSelectedAsArchive(entries: StorageEntryDto[]): void {
    if (entries.length === 0) {
      return;
    }

    const accessToken = this.sessionService.readAccessToken();
    if (!accessToken) {
      this.redirectToLogin();
      return;
    }

    this.isBatchDownloadInProgress = true;
    this.storageErrorMessage = '';
    this.cdr.detectChanges();

    this.storageApiService
      .downloadBatch(
        '',
        accessToken,
        entries.map((entry) => ({
          entryType: entry.entryType,
          resourceId: entry.id
        }))
      )
      .pipe(finalize(() => {
        this.isBatchDownloadInProgress = false;
        this.cdr.detectChanges();
      }))
      .subscribe({
        next: (response: HttpResponse<Blob>) => {
          const archiveBlob = response.body;
          if (!archiveBlob) {
            this.storageErrorMessage = 'Batch download returned an empty archive';
            this.cdr.detectChanges();
            return;
          }

          const fileName = this.extractBatchDownloadName(response, entries.length);
          const downloadUrl = URL.createObjectURL(archiveBlob);
          const anchor = document.createElement('a');
          anchor.href = downloadUrl;
          anchor.download = fileName;
          anchor.rel = 'noopener noreferrer';
          document.body.appendChild(anchor);
          anchor.click();
          anchor.remove();
          window.setTimeout(() => URL.revokeObjectURL(downloadUrl), 30_000);
          this.cdr.detectChanges();
        },
        error: (error: unknown) => {
          this.storageErrorMessage = this.extractError(error, 'Failed to download selected items');

          if (error instanceof HttpErrorResponse && error.status === 401) {
            this.redirectToLogin();
            return;
          }

          this.cdr.detectChanges();
        }
      });
  }

  private matchesTypeFilter(entry: StorageEntryDto): boolean {
    if (this.typeFilter === 'all') {
      return true;
    }

    if (this.typeFilter === 'folder') {
      return entry.entryType === 'folder';
    }

    if (entry.entryType !== 'file') {
      return false;
    }

    if (this.typeFilter === 'file-any') {
      return true;
    }

    const extension = this.fileExtension(entry.name);
    if (!extension) {
      return false;
    }

    switch (this.typeFilter) {
      case 'image':
        return IMAGE_EXTENSIONS.has(extension);
      case 'document':
        return DOCUMENT_EXTENSIONS.has(extension);
      case 'pdf':
        return PDF_EXTENSIONS.has(extension);
      case 'spreadsheet':
        return SPREADSHEET_EXTENSIONS.has(extension);
      case 'presentation':
        return PRESENTATION_EXTENSIONS.has(extension);
      case 'archive':
        return ARCHIVE_EXTENSIONS.has(extension);
      case 'audio':
        return AUDIO_EXTENSIONS.has(extension);
      case 'video':
        return VIDEO_EXTENSIONS.has(extension);
      case 'code':
        return CODE_EXTENSIONS.has(extension);
      default:
        return true;
    }
  }

  private modifiedSinceThreshold(): number | null {
    const now = Date.now();

    switch (this.modifiedFilter) {
      case 'last-day':
        return now - 24 * 60 * 60 * 1000;
      case 'last-week':
        return now - 7 * 24 * 60 * 60 * 1000;
      case 'last-month':
        return now - 30 * 24 * 60 * 60 * 1000;
      case 'last-year':
        return now - 365 * 24 * 60 * 60 * 1000;
      default:
        return null;
    }
  }

  private fileExtension(fileName: string): string | null {
    const normalizedName = fileName.trim().toLowerCase();
    const dotIndex = normalizedName.lastIndexOf('.');

    if (dotIndex <= 0 || dotIndex >= normalizedName.length - 1) {
      return null;
    }

    return normalizedName.slice(dotIndex + 1);
  }

  private compareNames(left: string, right: string, order: NameSortOrder): number {
    const direction = order === 'asc' ? 1 : -1;
    const compared = left.localeCompare(right, undefined, {
      sensitivity: 'base',
      numeric: true
    });

    return compared * direction;
  }

  private compareNullableNumbers(
    left: number | null,
    right: number | null,
    order: NameSortOrder
  ): number {
    const leftValid = left !== null && Number.isFinite(left);
    const rightValid = right !== null && Number.isFinite(right);

    if (!leftValid && !rightValid) {
      return 0;
    }

    if (!leftValid) {
      return 1;
    }

    if (!rightValid) {
      return -1;
    }

    if (left === right) {
      return 0;
    }

    if (order === 'asc') {
      return left < right ? -1 : 1;
    }

    return left > right ? -1 : 1;
  }

  private resetFiltersForNavigation(): void {
    this.typeFilter = 'all';
    this.modifiedFilter = 'any';
    this.peopleSearchTerm = '';
    this.selectedPeopleUserIds.clear();
    this.activeFilterMenu = null;
  }

  private isDescendantPath(path: string, ancestor: string): boolean {
    const normalizedPath = this.normalizeStoragePath(path);
    const normalizedAncestor = this.normalizeStoragePath(ancestor);

    if (normalizedAncestor === '/') {
      return normalizedPath !== '/';
    }

    return normalizedPath.length > normalizedAncestor.length
      && normalizedPath.startsWith(normalizedAncestor)
      && normalizedPath.charAt(normalizedAncestor.length) === '/';
  }

  private normalizeStoragePath(path: string): string {
    const trimmed = path.trim();
    if (trimmed.length === 0 || trimmed === '/') {
      return '/';
    }

    const normalized = trimmed.replace(/\/+/g, '/');
    if (normalized.startsWith('/')) {
      return normalized;
    }

    return `/${normalized}`;
  }

  private redirectToLogin(): void {
    this.sessionService.clearSession();
    this.router.navigate(['/login']);
  }

  private reloadCurrentFolder(): void {
    if (this.currentFolderId !== null) {
      this.loadStorageByFolderId(this.currentFolderId);
      return;
    }

    this.loadStorageByPath(this.currentPath, false);
  }

  private navigateToFolderRoute(folderId: number): void {
    const extras = this.isSharedContext
      ? { queryParams: { source: 'shared' } }
      : undefined;
    const targetTree = this.router.createUrlTree(['/app/storage/folders', folderId], extras);
    const targetUrl = this.router.serializeUrl(targetTree);

    if (this.router.url === targetUrl) {
      return;
    }

    this.router.navigate(['/app/storage/folders', folderId], extras);
  }

  private get isSharedContext(): boolean {
    return this.navigationSource === 'shared' || this.currentPrivilege !== 'owner';
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

    return target.closest('[contenteditable="true"]') !== null;
  }

  private extractBatchDownloadName(response: HttpResponse<Blob>, selectionSize: number): string {
    const contentDisposition = response.headers.get('content-disposition');
    if (contentDisposition) {
      const utf8Match = contentDisposition.match(/filename\*=UTF-8''([^;]+)/i);
      if (utf8Match?.[1]) {
        try {
          const decoded = decodeURIComponent(utf8Match[1]);
          const normalized = decoded.trim().replace(/[\\/]/g, '_');
          if (normalized.length > 0) {
            return normalized;
          }
        } catch {
          // Keep fallback handling below.
        }
      }

      const quotedMatch = contentDisposition.match(/filename="([^"]+)"/i);
      const plainMatch = contentDisposition.match(/filename=([^;]+)/i);
      const candidate = quotedMatch?.[1] ?? plainMatch?.[1] ?? '';
      const normalized = candidate.trim().replace(/[\\/]/g, '_');
      if (normalized.length > 0) {
        return normalized;
      }
    }

    const unixSeconds = Math.floor(Date.now() / 1000);
    return `pcloud-batch-${selectionSize}-${unixSeconds}.zip`;
  }

  private triggerUiRefresh(): void {
    const view = this.cdr as ViewRef;
    if (view.destroyed) {
      return;
    }

    this.cdr.detectChanges();
  }
}
