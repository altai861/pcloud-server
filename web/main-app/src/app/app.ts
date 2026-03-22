import { Component, OnInit } from '@angular/core';
import { RouterOutlet } from '@angular/router';

import { I18nService } from './services/i18n.service';
import { ThemeService } from './services/theme.service';

@Component({
  selector: 'app-root',
  imports: [RouterOutlet],
  templateUrl: './app.html',
  styleUrl: './app.css'
})
export class App implements OnInit {
  constructor(
    private readonly themeService: ThemeService,
    private readonly i18nService: I18nService
  ) {}

  ngOnInit(): void {
    this.i18nService.initializeLanguage();
    this.themeService.initializeTheme();
  }
}
