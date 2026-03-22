import { CommonModule } from '@angular/common';
import { ChangeDetectorRef, Component, ElementRef, HostListener, OnDestroy, OnInit, ViewChild } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { NavigationEnd, Router, RouterLink, RouterLinkActive, RouterOutlet } from '@angular/router';
import { Subscription, filter } from 'rxjs';
import { TPipe } from '../../pipes/t.pipe';

import { AuthUserDto } from '../../dto/auth-user.dto';
import { AuthApiService } from '../../services/auth-api.service';
import { ClientSessionService } from '../../services/client-session.service';
import { I18nService } from '../../services/i18n.service';
import { ProfileImageService } from '../../services/profile-image.service';
import { StorageSidebarAction, StorageSidebarActionsService } from '../../services/storage-sidebar-actions.service';
import { ThemeService } from '../../services/theme.service';
import { WorkspaceSearchService } from '../../services/workspace-search.service';

@Component({
  selector: 'app-workspace-shell',
  imports: [CommonModule, FormsModule, RouterLink, RouterLinkActive, RouterOutlet, TPipe],
  templateUrl: './workspace-shell.component.html',
  styleUrl: './workspace-shell.component.css'
})
export class WorkspaceShellComponent implements OnInit, OnDestroy {
  @ViewChild('uploadFileInput') uploadFileInput: ElementRef<HTMLInputElement> | null = null;

  private profileImageSub: Subscription | null = null;
  private usageChangedSub: Subscription | null = null;
  private routerEventsSub: Subscription | null = null;

  currentUser: AuthUserDto | null = null;
  profileImageSrc: string | null = null;
  searchInput = '';
  isDarkMode = false;
  isNewMenuOpen = false;
  currentLanguage: 'en' | 'mn' = 'en';

  get storageUsagePercent(): number {
    const quota = this.currentUser?.storageQuotaBytes ?? 0;
    const used = this.currentUser?.storageUsedBytes ?? 0;

    if (!Number.isFinite(quota) || quota <= 0 || !Number.isFinite(used) || used <= 0) {
      return 0;
    }

    return Math.min(100, Math.max(0, (used / quota) * 100));
  }

  constructor(
    private readonly authApiService: AuthApiService,
    private readonly sessionService: ClientSessionService,
    private readonly i18nService: I18nService,
    private readonly profileImageService: ProfileImageService,
    private readonly storageSidebarActions: StorageSidebarActionsService,
    private readonly themeService: ThemeService,
    private readonly searchService: WorkspaceSearchService,
    private readonly router: Router,
    private readonly cdr: ChangeDetectorRef
  ) {}

  ngOnInit(): void {
    this.isDarkMode = this.themeService.getCurrentTheme() === 'dark';
    this.currentLanguage = this.i18nService.getCurrentLanguage();
    this.profileImageSub = this.profileImageService.profileImageSrc$.subscribe((src) => {
      this.profileImageSrc = src;
      this.cdr.detectChanges();
    });
    this.usageChangedSub = this.storageSidebarActions.usageChanged$.subscribe(() => {
      this.loadCurrentUser();
    });
    this.routerEventsSub = this.router.events
      .pipe(filter((event) => event instanceof NavigationEnd))
      .subscribe(() => {
        this.syncSearchInputFromRoute();
      });
    this.syncSearchInputFromRoute();
    this.loadCurrentUser();
  }

  ngOnDestroy(): void {
    this.profileImageSub?.unsubscribe();
    this.profileImageSub = null;
    this.usageChangedSub?.unsubscribe();
    this.usageChangedSub = null;
    this.routerEventsSub?.unsubscribe();
    this.routerEventsSub = null;
  }

  onSearchChange(value: string): void {
    this.searchInput = value;
  }

  onSearchSubmit(event?: Event): void {
    event?.preventDefault();
    const normalizedQuery = this.searchInput.trim();
    this.searchService.setSearchTerm(normalizedQuery);

    const queryParams = normalizedQuery.length > 0 ? { q: normalizedQuery } : {};
    this.router.navigate(['/app/search'], { queryParams });
  }

  onProfileClick(): void {
    this.router.navigate(['/app/profile']);
  }

  toggleTheme(): void {
    this.isDarkMode = this.themeService.toggleTheme() === 'dark';
  }

  toggleLanguage(): void {
    this.currentLanguage = this.i18nService.toggleLanguage();
    this.cdr.detectChanges();
  }

  toggleNewMenu(event: MouseEvent): void {
    event.stopPropagation();
    this.isNewMenuOpen = !this.isNewMenuOpen;
  }

  onCreateFolderClick(event: MouseEvent): void {
    event.stopPropagation();
    this.isNewMenuOpen = false;
    this.dispatchStorageAction({ type: 'create-folder' });
  }

  onUploadFileClick(event: MouseEvent): void {
    event.stopPropagation();
    this.isNewMenuOpen = false;
    this.uploadFileInput?.nativeElement.click();
  }

  onUploadFileSelected(event: Event): void {
    const input = event.target as HTMLInputElement;
    const files = Array.from(input.files ?? []);

    if (input) {
      input.value = '';
    }

    if (files.length === 0) {
      return;
    }

    this.dispatchStorageAction({ type: 'upload-files', files });
  }

  @HostListener('document:click', ['$event'])
  onDocumentClick(event: MouseEvent): void {
    if (!this.isNewMenuOpen) {
      return;
    }

    const clickedInside = (event.target as HTMLElement | null)?.closest('.new-menu-wrap');
    if (!clickedInside) {
      this.isNewMenuOpen = false;
    }
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

  private loadCurrentUser(): void {
    const token = this.sessionService.readAccessToken();

    if (!token) {
      this.profileImageService.clear();
      this.router.navigate(['/login']);
      return;
    }

    this.authApiService.me('', token).subscribe({
      next: (response) => {
        this.currentUser = response.user;
        if (response.user.profileImageUrl) {
          this.loadProfileImage(token);
        } else {
          this.profileImageService.clear();
        }
        this.cdr.detectChanges();
      },
      error: () => {
        this.profileImageService.clear();
        this.sessionService.clearSession();
        this.router.navigate(['/login']);
      }
    });
  }

  private dispatchStorageAction(action: StorageSidebarAction): void {
    if (this.router.url.startsWith('/app/storage')) {
      this.storageSidebarActions.emit(action);
      return;
    }

    this.storageSidebarActions.queue(action);
    this.router.navigate(['/app/storage']);
  }

  private loadProfileImage(token: string): void {
    this.authApiService.getProfileImage('', token).subscribe({
      next: (blob) => {
        this.profileImageService.setFromBlob(blob);
      },
      error: () => {
        this.profileImageService.clear();
      }
    });
  }

  private syncSearchInputFromRoute(): void {
    const urlTree = this.router.parseUrl(this.router.url);
    const primarySegments = urlTree.root.children['primary']?.segments ?? [];
    const primaryPath = primarySegments.map((segment) => segment.path).join('/');

    if (primaryPath === 'app/search') {
      const queryValue = urlTree.queryParams['q'];
      this.searchInput = typeof queryValue === 'string' ? queryValue : '';
      this.searchService.setSearchTerm(this.searchInput);
      this.cdr.detectChanges();
      return;
    }

    this.searchService.setSearchTerm('');
  }
}
