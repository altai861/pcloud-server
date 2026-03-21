import { HttpClient, HttpHeaders, HttpParams } from '@angular/common/http';
import { Injectable } from '@angular/core';
import { Observable } from 'rxjs';

import { StorageListResponseDto } from '../dto/storage-list-response.dto';

@Injectable({
  providedIn: 'root'
})
export class StorageApiService {
  constructor(private readonly http: HttpClient) {}

  list(
    apiBaseUrl: string,
    accessToken: string,
    path: string,
    search: string
  ): Observable<StorageListResponseDto> {
    let params = new HttpParams();

    if (path.trim().length > 0) {
      params = params.set('path', path.trim());
    }

    if (search.trim().length > 0) {
      params = params.set('q', search.trim());
    }

    return this.http.get<StorageListResponseDto>(
      this.buildUrl(apiBaseUrl, '/api/client/storage/list'),
      {
        headers: this.authHeaders(accessToken),
        params
      }
    );
  }

  private authHeaders(accessToken: string): HttpHeaders {
    return new HttpHeaders({
      Authorization: `Bearer ${accessToken}`
    });
  }

  private buildUrl(apiBaseUrl: string, path: string): string {
    const normalized = apiBaseUrl.trim().replace(/\/+$/, '');

    if (normalized.length === 0) {
      return path;
    }

    return `${normalized}${path}`;
  }
}
