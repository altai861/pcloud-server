import { CommonModule } from '@angular/common';
import { ChangeDetectorRef, Component, ElementRef, HostListener, OnDestroy, OnInit, ViewChild } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { Router, RouterLink, RouterLinkActive, RouterOutlet } from '@angular/router';
import { Subscription } from 'rxjs';

import { AuthUserDto } from '../../dto/auth-user.dto';
import { AuthApiService } from '../../services/auth-api.service';
import { ClientSessionService } from '../../services/client-session.service';
import { ProfileImageService } from '../../services/profile-image.service';
import { StorageSidebarAction, StorageSidebarActionsService } from '../../services/storage-sidebar-actions.service';
import { ThemeService } from '../../services/theme.service';
import { WorkspaceSearchService } from '../../services/workspace-search.service';

@Component({
  selector: 'app-workspace-shell',
  imports: [CommonModule, FormsModule, RouterLink, RouterLinkActive, RouterOutlet],
  templateUrl: './workspace-shell.component.html',
  styleUrl: './workspace-shell.component.css'
})
export class WorkspaceShellComponent implements OnInit, OnDestroy {
  @ViewChild('uploadFileInput') uploadFileInput: ElementRef<HTMLInputElement> | null = null;

  private profileImageSub: Subscription | null = null;

  currentUser: AuthUserDto | null = null;
  profileImageSrc: string | null = null;
  searchInput = '';
  isDarkMode = false;
  isNewMenuOpen = false;

  constructor(
    private readonly authApiService: AuthApiService,
    private readonly sessionService: ClientSessionService,
    private readonly profileImageService: ProfileImageService,
    private readonly storageSidebarActions: StorageSidebarActionsService,
    private readonly themeService: ThemeService,
    private readonly searchService: WorkspaceSearchService,
    private readonly router: Router,
    private readonly cdr: ChangeDetectorRef
  ) {}

  ngOnInit(): void {
    this.isDarkMode = this.themeService.getCurrentTheme() === 'dark';
    this.profileImageSub = this.profileImageService.profileImageSrc$.subscribe((src) => {
      this.profileImageSrc = src;
      this.cdr.detectChanges();
    });
    this.loadCurrentUser();
  }

  ngOnDestroy(): void {
    this.profileImageSub?.unsubscribe();
    this.profileImageSub = null;
  }

  onSearchChange(value: string): void {
    this.searchInput = value;
    this.searchService.setSearchTerm(value);
  }

  onProfileClick(): void {
    this.router.navigate(['/app/profile']);
  }

  toggleTheme(): void {
    this.isDarkMode = this.themeService.toggleTheme() === 'dark';
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
    const file = input.files?.[0];

    if (input) {
      input.value = '';
    }

    if (!file) {
      return;
    }

    this.dispatchStorageAction({ type: 'upload-file', file });
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
}
