import { HttpClient, HttpEvent, HttpHeaders, HttpParams } from '@angular/common/http';
import { Injectable } from '@angular/core';
import { Observable } from 'rxjs';

import { ShareMutationResponseDto } from '../dto/share-mutation-response.dto';
import { ShareableUsersResponseDto } from '../dto/shareable-users-response.dto';
import { SharedPermissionsResponseDto } from '../dto/shared-permissions-response.dto';
import { SharedResourcesListResponseDto } from '../dto/shared-resources-list-response.dto';
import { StorageDeleteResponseDto } from '../dto/storage-delete-response.dto';
import { StorageFileMetadataDto } from '../dto/storage-file-metadata.dto';
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
    search: string,
    folderId: number | null = null
  ): Observable<StorageListResponseDto> {
    let params = new HttpParams();

    if (path.trim().length > 0) {
      params = params.set('path', path.trim());
    }

    if (search.trim().length > 0) {
      params = params.set('q', search.trim());
    }

    if (folderId !== null && Number.isFinite(folderId)) {
      params = params.set('folderId', String(Math.trunc(folderId)));
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
    parentFolderId: number | null,
    name: string
  ): Observable<StorageMutationResponseDto> {
    const payload: {
      parentPath: string;
      parentFolderId?: number;
      name: string;
    } = {
      parentPath,
      name
    };

    if (parentFolderId !== null && Number.isFinite(parentFolderId)) {
      payload.parentFolderId = Math.trunc(parentFolderId);
    }

    return this.http.post<StorageMutationResponseDto>(
      this.buildUrl(apiBaseUrl, '/api/client/storage/folders'),
      payload,
      {
        headers: this.authHeaders(accessToken)
      }
    );
  }

  renameFolder(
    apiBaseUrl: string,
    accessToken: string,
    path: string,
    newName: string,
    resourceId: number | null = null
  ): Observable<StorageMutationResponseDto> {
    const payload: {
      path: string;
      newName: string;
      resourceId?: number;
    } = {
      path,
      newName
    };

    if (resourceId !== null && Number.isFinite(resourceId)) {
      payload.resourceId = Math.trunc(resourceId);
    }

    return this.http.put<StorageMutationResponseDto>(
      this.buildUrl(apiBaseUrl, '/api/client/storage/folders'),
      payload,
      {
        headers: this.authHeaders(accessToken)
      }
    );
  }

  renameFile(
    apiBaseUrl: string,
    accessToken: string,
    path: string,
    newName: string,
    resourceId: number | null = null
  ): Observable<StorageMutationResponseDto> {
    const payload: {
      path: string;
      newName: string;
      resourceId?: number;
    } = {
      path,
      newName
    };

    if (resourceId !== null && Number.isFinite(resourceId)) {
      payload.resourceId = Math.trunc(resourceId);
    }

    return this.http.put<StorageMutationResponseDto>(
      this.buildUrl(apiBaseUrl, '/api/client/storage/files'),
      payload,
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
    path: string,
    folderId: number | null = null
  ): Observable<StorageFolderMetadataDto> {
    let params = new HttpParams();

    if (path.trim().length > 0) {
      params = params.set('path', path.trim());
    }

    if (folderId !== null && Number.isFinite(folderId)) {
      params = params.set('folderId', String(Math.trunc(folderId)));
    }

    return this.http.get<StorageFolderMetadataDto>(
      this.buildUrl(apiBaseUrl, '/api/client/storage/folders/metadata'),
      {
        headers: this.authHeaders(accessToken),
        params
      }
    );
  }

  fileMetadata(
    apiBaseUrl: string,
    accessToken: string,
    fileId: number
  ): Observable<StorageFileMetadataDto> {
    const params = new HttpParams().set('fileId', String(Math.trunc(fileId)));

    return this.http.get<StorageFileMetadataDto>(
      this.buildUrl(apiBaseUrl, '/api/client/storage/files/metadata'),
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
    folderId: number | null,
    file: File
  ): Observable<HttpEvent<StorageMutationResponseDto>> {
    const formData = new FormData();
    formData.append('path', path);
    if (folderId !== null && Number.isFinite(folderId)) {
      formData.append('folderId', String(Math.trunc(folderId)));
    }
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
    path: string,
    fileId: number | null = null
  ): string {
    const params = new URLSearchParams();

    if (fileId !== null && Number.isFinite(fileId)) {
      params.set('fileId', String(Math.trunc(fileId)));
    }

    if (path.trim().length > 0) {
      params.set('path', path.trim());
    }

    params.set('accessToken', accessToken.trim());

    return `${this.buildUrl(apiBaseUrl, '/api/client/storage/files/download')}?${params.toString()}`;
  }

  buildUserProfileImageUrl(
    apiBaseUrl: string,
    accessToken: string,
    userId: number
  ): string {
    const params = new URLSearchParams();
    params.set('userId', String(Math.trunc(userId)));
    params.set('accessToken', accessToken.trim());

    return `${this.buildUrl(apiBaseUrl, '/api/client/users/profile-image')}?${params.toString()}`;
  }

  listShared(
    apiBaseUrl: string,
    accessToken: string,
    search: string
  ): Observable<SharedResourcesListResponseDto> {
    let params = new HttpParams();

    if (search.trim().length > 0) {
      params = params.set('q', search.trim());
    }

    return this.http.get<SharedResourcesListResponseDto>(
      this.buildUrl(apiBaseUrl, '/api/client/storage/shared/list'),
      {
        headers: this.authHeaders(accessToken),
        params
      }
    );
  }

  listSharePermissions(
    apiBaseUrl: string,
    accessToken: string,
    entryType: 'folder' | 'file',
    resourceId: number
  ): Observable<SharedPermissionsResponseDto> {
    const params = new HttpParams()
      .set('entryType', entryType)
      .set('resourceId', String(Math.trunc(resourceId)));

    return this.http.get<SharedPermissionsResponseDto>(
      this.buildUrl(apiBaseUrl, '/api/client/storage/shares'),
      {
        headers: this.authHeaders(accessToken),
        params
      }
    );
  }

  upsertSharePermission(
    apiBaseUrl: string,
    accessToken: string,
    entryType: 'folder' | 'file',
    resourceId: number,
    targetUserId: number,
    privilegeType: 'viewer' | 'editor'
  ): Observable<ShareMutationResponseDto> {
    return this.http.put<ShareMutationResponseDto>(
      this.buildUrl(apiBaseUrl, '/api/client/storage/shares'),
      {
        entryType,
        resourceId: Math.trunc(resourceId),
        targetUserId: Math.trunc(targetUserId),
        privilegeType
      },
      {
        headers: this.authHeaders(accessToken)
      }
    );
  }

  removeSharePermission(
    apiBaseUrl: string,
    accessToken: string,
    entryType: 'folder' | 'file',
    resourceId: number,
    targetUserId: number
  ): Observable<ShareMutationResponseDto> {
    const params = new HttpParams()
      .set('entryType', entryType)
      .set('resourceId', String(Math.trunc(resourceId)))
      .set('targetUserId', String(Math.trunc(targetUserId)));

    return this.http.delete<ShareMutationResponseDto>(
      this.buildUrl(apiBaseUrl, '/api/client/storage/shares'),
      {
        headers: this.authHeaders(accessToken),
        params
      }
    );
  }

  searchShareableUsers(
    apiBaseUrl: string,
    accessToken: string,
    search: string
  ): Observable<ShareableUsersResponseDto> {
    let params = new HttpParams();

    if (search.trim().length > 0) {
      params = params.set('q', search.trim());
    }

    return this.http.get<ShareableUsersResponseDto>(
      this.buildUrl(apiBaseUrl, '/api/client/storage/shares/users'),
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
