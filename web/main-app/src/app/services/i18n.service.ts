import { Injectable } from '@angular/core';
import { BehaviorSubject } from 'rxjs';

import { AppLanguage, MAIN_APP_TRANSLATIONS } from '../i18n/translations';

const LANGUAGE_STORAGE_KEY = 'pcloud_main_app_language';

@Injectable({
  providedIn: 'root'
})
export class I18nService {
  private readonly languageSubject = new BehaviorSubject<AppLanguage>('en');
  readonly language$ = this.languageSubject.asObservable();

  initializeLanguage(): AppLanguage {
    const persisted = localStorage.getItem(LANGUAGE_STORAGE_KEY);
    const normalized = persisted === 'mn' || persisted === 'en' ? persisted : 'en';
    this.languageSubject.next(normalized);
    return normalized;
  }

  getCurrentLanguage(): AppLanguage {
    return this.languageSubject.value;
  }

  setLanguage(nextLanguage: AppLanguage): void {
    if (nextLanguage === this.languageSubject.value) {
      return;
    }

    this.languageSubject.next(nextLanguage);
    localStorage.setItem(LANGUAGE_STORAGE_KEY, nextLanguage);
  }

  toggleLanguage(): AppLanguage {
    const nextLanguage: AppLanguage = this.languageSubject.value === 'en' ? 'mn' : 'en';
    this.setLanguage(nextLanguage);
    return nextLanguage;
  }

  t(key: string, params?: Record<string, string | number | null | undefined>): string {
    const currentLanguage = this.languageSubject.value;
    const currentDict = MAIN_APP_TRANSLATIONS[currentLanguage];
    const fallbackDict = MAIN_APP_TRANSLATIONS.en;
    const template = currentDict[key] ?? fallbackDict[key] ?? key;

    if (!params) {
      return template;
    }

    return Object.entries(params).reduce((result, [token, value]) => {
      const normalized = value === null || value === undefined ? '' : String(value);
      return result.replace(new RegExp(`\\{${token}\\}`, 'g'), normalized);
    }, template);
  }
}
