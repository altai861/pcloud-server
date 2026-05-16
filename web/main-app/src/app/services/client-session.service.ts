import { Injectable } from '@angular/core';

@Injectable({
  providedIn: 'root'
})
export class ClientSessionService {
  private readonly defaultAccessTokenStorageKey = 'pcloud.main.accessToken';

  readAccessToken(): string | null {
    if (typeof localStorage === 'undefined') {
      return null;
    }

    const value = localStorage.getItem(this.accessTokenStorageKey());
    if (!value || value.trim().length === 0) {
      return null;
    }

    return value;
  }

  setSession(accessToken: string): void {
    if (typeof localStorage === 'undefined') {
      return;
    }

    localStorage.setItem(this.accessTokenStorageKey(), accessToken);
  }

  clearSession(): void {
    if (typeof localStorage === 'undefined') {
      return;
    }

    localStorage.removeItem(this.accessTokenStorageKey());
  }

  private accessTokenStorageKey(): string {
    if (typeof window === 'undefined') {
      return this.defaultAccessTokenStorageKey;
    }

    const relayPrefix = window.location.pathname.match(/^\/d\/[^/]+/);
    if (!relayPrefix) {
      return this.defaultAccessTokenStorageKey;
    }

    return `${this.defaultAccessTokenStorageKey}:${relayPrefix[0]}`;
  }
}
