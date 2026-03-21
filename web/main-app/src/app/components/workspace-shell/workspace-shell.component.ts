import { CommonModule } from '@angular/common';
import { ChangeDetectorRef, Component, OnInit } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { Router, RouterLink, RouterLinkActive, RouterOutlet } from '@angular/router';

import { AuthUserDto } from '../../dto/auth-user.dto';
import { AuthApiService } from '../../services/auth-api.service';
import { ClientSessionService } from '../../services/client-session.service';
import { ThemeService } from '../../services/theme.service';
import { WorkspaceSearchService } from '../../services/workspace-search.service';

@Component({
  selector: 'app-workspace-shell',
  imports: [CommonModule, FormsModule, RouterLink, RouterLinkActive, RouterOutlet],
  templateUrl: './workspace-shell.component.html',
  styleUrl: './workspace-shell.component.css'
})
export class WorkspaceShellComponent implements OnInit {
  currentUser: AuthUserDto | null = null;
  searchInput = '';
  isDarkMode = false;

  constructor(
    private readonly authApiService: AuthApiService,
    private readonly sessionService: ClientSessionService,
    private readonly themeService: ThemeService,
    private readonly searchService: WorkspaceSearchService,
    private readonly router: Router,
    private readonly cdr: ChangeDetectorRef
  ) {}

  ngOnInit(): void {
    this.isDarkMode = this.themeService.getCurrentTheme() === 'dark';
    this.loadCurrentUser();
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
      this.router.navigate(['/login']);
      return;
    }

    this.authApiService.me('', token).subscribe({
      next: (response) => {
        this.currentUser = response.user;
        this.cdr.detectChanges();
      },
      error: () => {
        this.sessionService.clearSession();
        this.router.navigate(['/login']);
      }
    });
  }
}
