import { HttpClient, HttpEvent, HttpHeaders, HttpParams } from '@angular/common/http';
import { Injectable } from '@angular/core';
import { Observable } from 'rxjs';

import { StorageListResponseDto } from '../dto/storage-list-response.dto';
import { StorageMutationResponseDto } from '../dto/storage-mutation-response.dto';

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

  createFolder(
    apiBaseUrl: string,
    accessToken: string,
    parentPath: string,
    name: string
  ): Observable<StorageMutationResponseDto> {
    return this.http.post<StorageMutationResponseDto>(
      this.buildUrl(apiBaseUrl, '/api/client/storage/folders'),
      {
        parentPath,
        name
      },
      {
        headers: this.authHeaders(accessToken)
      }
    );
  }

  uploadFile(
    apiBaseUrl: string,
    accessToken: string,
    path: string,
    file: File
  ): Observable<HttpEvent<StorageMutationResponseDto>> {
    const formData = new FormData();
    formData.append('path', path);
    formData.append('file', file, file.name);

    return this.http.post<StorageMutationResponseDto>(
      this.buildUrl(apiBaseUrl, '/api/client/storage/files/upload'),
      formData,
      {
        headers: this.authHeaders(accessToken),
        reportProgress: true,
        observe: 'events'
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
