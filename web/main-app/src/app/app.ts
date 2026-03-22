import { Component, OnDestroy, OnInit } from '@angular/core';
import { Title } from '@angular/platform-browser';
import { NavigationEnd, Router, RouterOutlet } from '@angular/router';
import { Subscription, filter } from 'rxjs';

import { I18nService } from './services/i18n.service';
import { ThemeService } from './services/theme.service';

@Component({
  selector: 'app-root',
  imports: [RouterOutlet],
  templateUrl: './app.html',
  styleUrl: './app.css'
})
export class App implements OnInit, OnDestroy {
  private routerEventsSub: Subscription | null = null;
  private languageSub: Subscription | null = null;

  constructor(
    private readonly themeService: ThemeService,
    private readonly i18nService: I18nService,
    private readonly router: Router,
    private readonly title: Title
  ) {}

  ngOnInit(): void {
    this.i18nService.initializeLanguage();
    this.themeService.initializeTheme();
    this.updatePageTitle();

    this.routerEventsSub = this.router.events
      .pipe(filter((event) => event instanceof NavigationEnd))
      .subscribe(() => {
        this.updatePageTitle();
      });

    this.languageSub = this.i18nService.language$.subscribe(() => {
      this.updatePageTitle();
    });
  }

  ngOnDestroy(): void {
    this.routerEventsSub?.unsubscribe();
    this.routerEventsSub = null;
    this.languageSub?.unsubscribe();
    this.languageSub = null;
  }

  private updatePageTitle(): void {
    const urlTree = this.router.parseUrl(this.router.url);
    const segments = urlTree.root.children['primary']?.segments.map((segment) => segment.path) ?? [];
    const titleKey = this.resolveTitleKey(segments);
    const pageTitle = this.i18nService.t(titleKey);
    this.title.setTitle(`${pageTitle} - PCloud`);
  }

  private resolveTitleKey(segments: string[]): string {
    if (segments.length === 0) {
      return 'tab.login';
    }

    if (segments[0] === 'login') {
      return 'tab.login';
    }

    if (segments[0] !== 'app') {
      return 'tab.workspace';
    }

    const page = segments[1] ?? '';

    switch (page) {
      case 'home':
        return 'tab.home';
      case 'storage':
        return segments[2] === 'files' ? 'tab.file' : 'tab.storage';
      case 'profile':
        return 'tab.profile';
      case 'starred':
        return 'tab.starred';
      case 'shared':
        return 'tab.shared';
      case 'search':
        return 'tab.search';
      case 'admin':
        return 'tab.admin';
      case 'trash':
        return 'tab.trash';
      default:
        return 'tab.workspace';
    }
  }
}
