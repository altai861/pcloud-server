import { CommonModule } from '@angular/common';
import { HttpErrorResponse } from '@angular/common/http';
import { ChangeDetectorRef, Component, HostListener, OnDestroy, OnInit } from '@angular/core';
import { Router } from '@angular/router';
import { finalize, Subscription } from 'rxjs';

import { ApiErrorResponseDto } from '../../dto/api-error-response.dto';
import { SharedResourceEntryDto } from '../../dto/shared-resource-entry.dto';
import { TPipe } from '../../pipes/t.pipe';
import { I18nService } from '../../services/i18n.service';
import { ClientSessionService } from '../../services/client-session.service';
import { StorageApiService } from '../../services/storage-api.service';

type FilterMenuKind = 'type' | 'people' | 'modified' | null;
type NameSortOrder = 'asc' | 'desc';
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
  selector: 'app-shared-page',
  imports: [CommonModule, TPipe],
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

  activeFilterMenu: FilterMenuKind = null;
  typeFilter: TypeFilterValue = 'all';
  modifiedFilter: ModifiedFilterValue = 'any';
  nameSortOrder: NameSortOrder = 'asc';
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
    private readonly i18nService: I18nService,
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

  get displayEntries(): SharedResourceEntryDto[] {
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
      filtered = filtered.filter((entry) => entry.dateSharedUnixMs >= modifiedThreshold);
    }

    const direction = this.nameSortOrder === 'asc' ? 1 : -1;
    return [...filtered].sort((left, right) => {
      const byName = left.name.localeCompare(right.name, undefined, {
        sensitivity: 'base',
        numeric: true
      });

      if (byName !== 0) {
        return byName * direction;
      }

      if (left.resourceType !== right.resourceType) {
        return left.resourceType === 'folder' ? -1 : 1;
      }

      return (left.resourceId - right.resourceId) * direction;
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
    this.cdr.detectChanges();
  }

  closeFilterMenu(): void {
    this.activeFilterMenu = null;
  }

  setTypeFilter(value: TypeFilterValue): void {
    this.typeFilter = value;
    this.cdr.detectChanges();
  }

  setModifiedFilter(value: ModifiedFilterValue): void {
    this.modifiedFilter = value;
    this.cdr.detectChanges();
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

    this.cdr.detectChanges();
  }

  clearPeopleSelection(): void {
    if (this.selectedPeopleUserIds.size === 0) {
      return;
    }

    this.selectedPeopleUserIds.clear();
    this.cdr.detectChanges();
  }

  clearAllFilters(): void {
    this.typeFilter = 'all';
    this.modifiedFilter = 'any';
    this.selectedPeopleUserIds.clear();
    this.peopleSearchTerm = '';
    this.activeFilterMenu = null;
    this.cdr.detectChanges();
  }

  toggleSortOrder(): void {
    this.nameSortOrder = this.nameSortOrder === 'asc' ? 'desc' : 'asc';
    this.cdr.detectChanges();
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

  @HostListener('document:click', ['$event'])
  onDocumentClick(event: MouseEvent): void {
    const target = event.target as HTMLElement | null;
    const clickedInsideMenu = !!target?.closest('.entry-context-menu');
    const clickedInsideFilters = !!target?.closest('.shared-filters');

    if (this.isEntryMenuOpen && !clickedInsideMenu) {
      this.closeEntryMenu();
      this.cdr.detectChanges();
    }

    if (this.activeFilterMenu !== null && !clickedInsideFilters) {
      this.closeFilterMenu();
      this.cdr.detectChanges();
    }
  }

  @HostListener('document:keydown.escape')
  onEscapePressed(): void {
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

  private matchesTypeFilter(entry: SharedResourceEntryDto): boolean {
    if (this.typeFilter === 'all') {
      return true;
    }

    if (this.typeFilter === 'folder') {
      return entry.resourceType === 'folder';
    }

    if (entry.resourceType !== 'file') {
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
