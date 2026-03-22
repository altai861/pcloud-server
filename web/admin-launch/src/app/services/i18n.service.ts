import { Injectable } from '@angular/core';
import { BehaviorSubject } from 'rxjs';

import { ADMIN_LAUNCH_TRANSLATIONS, AppLanguage } from '../i18n/translations';

const LANGUAGE_STORAGE_KEY = 'pcloud_admin_launch_language';

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

  toggleLanguage(): AppLanguage {
    const nextLanguage: AppLanguage = this.languageSubject.value === 'en' ? 'mn' : 'en';
    this.languageSubject.next(nextLanguage);
    localStorage.setItem(LANGUAGE_STORAGE_KEY, nextLanguage);
    return nextLanguage;
  }

  t(key: string, params?: Record<string, string | number | null | undefined>): string {
    const currentLanguage = this.languageSubject.value;
    const currentDict = ADMIN_LAUNCH_TRANSLATIONS[currentLanguage];
    const fallbackDict = ADMIN_LAUNCH_TRANSLATIONS.en;
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
