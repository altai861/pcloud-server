import { HttpClient, HttpEvent, HttpHeaders, HttpParams } from '@angular/common/http';
import { Injectable } from '@angular/core';
import { Observable } from 'rxjs';

import { StorageDeleteResponseDto } from '../dto/storage-delete-response.dto';
import { StorageFolderMetadataDto } from '../dto/storage-folder-metadata.dto';
import { StorageListResponseDto } from '../dto/storage-list-response.dto';
import { StorageMutationResponseDto } from '../dto/storage-mutation-response.dto';
import { StorageRestoreResponseDto } from '../dto/storage-restore-response.dto';

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

  listTrash(
    apiBaseUrl: string,
    accessToken: string,
    search: string
  ): Observable<StorageListResponseDto> {
    let params = new HttpParams();

    if (search.trim().length > 0) {
      params = params.set('q', search.trim());
    }

    return this.http.get<StorageListResponseDto>(
      this.buildUrl(apiBaseUrl, '/api/client/storage/trash/list'),
      {
        headers: this.authHeaders(accessToken),
        params
      }
    );
  }

  listStarred(
    apiBaseUrl: string,
    accessToken: string,
    search: string
  ): Observable<StorageListResponseDto> {
    let params = new HttpParams();

    if (search.trim().length > 0) {
      params = params.set('q', search.trim());
    }

    return this.http.get<StorageListResponseDto>(
      this.buildUrl(apiBaseUrl, '/api/client/storage/starred/list'),
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

  renameFolder(
    apiBaseUrl: string,
    accessToken: string,
    path: string,
    newName: string
  ): Observable<StorageMutationResponseDto> {
    return this.http.put<StorageMutationResponseDto>(
      this.buildUrl(apiBaseUrl, '/api/client/storage/folders'),
      {
        path,
        newName
      },
      {
        headers: this.authHeaders(accessToken)
      }
    );
  }

  renameFile(
    apiBaseUrl: string,
    accessToken: string,
    path: string,
    newName: string
  ): Observable<StorageMutationResponseDto> {
    return this.http.put<StorageMutationResponseDto>(
      this.buildUrl(apiBaseUrl, '/api/client/storage/files'),
      {
        path,
        newName
      },
      {
        headers: this.authHeaders(accessToken)
      }
    );
  }

  setStarred(
    apiBaseUrl: string,
    accessToken: string,
    path: string,
    entryType: 'folder' | 'file',
    starred: boolean
  ): Observable<StorageMutationResponseDto> {
    return this.http.put<StorageMutationResponseDto>(
      this.buildUrl(apiBaseUrl, '/api/client/storage/starred'),
      {
        path,
        entryType,
        starred
      },
      {
        headers: this.authHeaders(accessToken)
      }
    );
  }

  folderMetadata(
    apiBaseUrl: string,
    accessToken: string,
    path: string
  ): Observable<StorageFolderMetadataDto> {
    let params = new HttpParams();

    if (path.trim().length > 0) {
      params = params.set('path', path.trim());
    }

    return this.http.get<StorageFolderMetadataDto>(
      this.buildUrl(apiBaseUrl, '/api/client/storage/folders/metadata'),
      {
        headers: this.authHeaders(accessToken),
        params
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

  deleteFile(
    apiBaseUrl: string,
    accessToken: string,
    path: string
  ): Observable<StorageDeleteResponseDto> {
    let params = new HttpParams();
    if (path.trim().length > 0) {
      params = params.set('path', path.trim());
    }

    return this.http.delete<StorageDeleteResponseDto>(
      this.buildUrl(apiBaseUrl, '/api/client/storage/files'),
      {
        headers: this.authHeaders(accessToken),
        params
      }
    );
  }

  deleteFolder(
    apiBaseUrl: string,
    accessToken: string,
    path: string
  ): Observable<StorageDeleteResponseDto> {
    let params = new HttpParams();
    if (path.trim().length > 0) {
      params = params.set('path', path.trim());
    }

    return this.http.delete<StorageDeleteResponseDto>(
      this.buildUrl(apiBaseUrl, '/api/client/storage/folders'),
      {
        headers: this.authHeaders(accessToken),
        params
      }
    );
  }

  permanentlyDeleteFile(
    apiBaseUrl: string,
    accessToken: string,
    path: string
  ): Observable<StorageDeleteResponseDto> {
    let params = new HttpParams();
    if (path.trim().length > 0) {
      params = params.set('path', path.trim());
    }

    return this.http.delete<StorageDeleteResponseDto>(
      this.buildUrl(apiBaseUrl, '/api/client/storage/trash/files'),
      {
        headers: this.authHeaders(accessToken),
        params
      }
    );
  }

  permanentlyDeleteFolder(
    apiBaseUrl: string,
    accessToken: string,
    path: string
  ): Observable<StorageDeleteResponseDto> {
    let params = new HttpParams();
    if (path.trim().length > 0) {
      params = params.set('path', path.trim());
    }

    return this.http.delete<StorageDeleteResponseDto>(
      this.buildUrl(apiBaseUrl, '/api/client/storage/trash/folders'),
      {
        headers: this.authHeaders(accessToken),
        params
      }
    );
  }

  restoreFile(
    apiBaseUrl: string,
    accessToken: string,
    path: string
  ): Observable<StorageRestoreResponseDto> {
    let params = new HttpParams();
    if (path.trim().length > 0) {
      params = params.set('path', path.trim());
    }

    return this.http.post<StorageRestoreResponseDto>(
      this.buildUrl(apiBaseUrl, '/api/client/storage/trash/files/restore'),
      null,
      {
        headers: this.authHeaders(accessToken),
        params
      }
    );
  }

  restoreFolder(
    apiBaseUrl: string,
    accessToken: string,
    path: string
  ): Observable<StorageRestoreResponseDto> {
    let params = new HttpParams();
    if (path.trim().length > 0) {
      params = params.set('path', path.trim());
    }

    return this.http.post<StorageRestoreResponseDto>(
      this.buildUrl(apiBaseUrl, '/api/client/storage/trash/folders/restore'),
      null,
      {
        headers: this.authHeaders(accessToken),
        params
      }
    );
  }

  buildFileDownloadUrl(
    apiBaseUrl: string,
    accessToken: string,
    path: string
  ): string {
    const params = new URLSearchParams();

    if (path.trim().length > 0) {
      params.set('path', path.trim());
    }

    params.set('accessToken', accessToken.trim());

    return `${this.buildUrl(apiBaseUrl, '/api/client/storage/files/download')}?${params.toString()}`;
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
