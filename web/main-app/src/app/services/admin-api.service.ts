import { HttpClient, HttpHeaders } from '@angular/common/http';
import { Injectable } from '@angular/core';
import { Observable } from 'rxjs';

import { AdminCreateUserRequestDto } from '../dto/admin-create-user-request.dto';
import { AdminCreateUserResponseDto } from '../dto/admin-create-user-response.dto';
import { AdminDeleteUserResponseDto } from '../dto/admin-delete-user-response.dto';
import { AdminUpdateUserRequestDto } from '../dto/admin-update-user-request.dto';
import { AdminUpdateUserResponseDto } from '../dto/admin-update-user-response.dto';
import { AdminUsersListResponseDto } from '../dto/admin-users-list-response.dto';

@Injectable({
  providedIn: 'root'
})
export class AdminApiService {
  constructor(private readonly http: HttpClient) {}

  listUsers(apiBaseUrl: string, accessToken: string): Observable<AdminUsersListResponseDto> {
    return this.http.get<AdminUsersListResponseDto>(
      this.buildUrl(apiBaseUrl, '/api/client/admin/users'),
      { headers: this.authHeaders(accessToken) }
    );
  }

  createUser(
    apiBaseUrl: string,
    accessToken: string,
    payload: AdminCreateUserRequestDto
  ): Observable<AdminCreateUserResponseDto> {
    return this.http.post<AdminCreateUserResponseDto>(
      this.buildUrl(apiBaseUrl, '/api/client/admin/users'),
      payload,
      { headers: this.authHeaders(accessToken) }
    );
  }

  updateUser(
    apiBaseUrl: string,
    accessToken: string,
    userId: number,
    payload: AdminUpdateUserRequestDto
  ): Observable<AdminUpdateUserResponseDto> {
    return this.http.put<AdminUpdateUserResponseDto>(
      this.buildUrl(apiBaseUrl, `/api/client/admin/users/${userId}`),
      payload,
      { headers: this.authHeaders(accessToken) }
    );
  }

  deleteUser(
    apiBaseUrl: string,
    accessToken: string,
    userId: number
  ): Observable<AdminDeleteUserResponseDto> {
    return this.http.delete<AdminDeleteUserResponseDto>(
      this.buildUrl(apiBaseUrl, `/api/client/admin/users/${userId}`),
      { headers: this.authHeaders(accessToken) }
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
