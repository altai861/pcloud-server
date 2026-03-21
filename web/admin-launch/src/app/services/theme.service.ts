import { DOCUMENT } from '@angular/common';
import { Inject, Injectable } from '@angular/core';

type ThemeMode = 'light' | 'dark';

@Injectable({
  providedIn: 'root'
})
export class ThemeService {
  private readonly storageKey = 'pcloud.theme';

  constructor(@Inject(DOCUMENT) private readonly document: Document) {}

  initializeTheme(): ThemeMode {
    const storedMode = this.readStoredTheme();
    const mode: ThemeMode = storedMode ?? 'light';
    this.applyTheme(mode);
    return mode;
  }

  getCurrentTheme(): ThemeMode {
    const mode = this.document.documentElement.getAttribute('data-theme');
    return mode === 'dark' ? 'dark' : 'light';
  }

  toggleTheme(): ThemeMode {
    const nextMode: ThemeMode = this.getCurrentTheme() === 'dark' ? 'light' : 'dark';
    this.applyTheme(nextMode);
    return nextMode;
  }

  private applyTheme(mode: ThemeMode): void {
    this.document.documentElement.setAttribute('data-theme', mode);

    try {
      localStorage.setItem(this.storageKey, mode);
    } catch {
      // Ignore storage errors and keep in-memory theme for current session.
    }
  }

  private readStoredTheme(): ThemeMode | null {
    try {
      const stored = localStorage.getItem(this.storageKey);
      if (stored === 'light' || stored === 'dark') {
        return stored;
      }
    } catch {
      // Ignore storage errors.
    }

    return null;
  }
}
