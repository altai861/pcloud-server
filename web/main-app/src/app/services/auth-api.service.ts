import { HttpClient, HttpHeaders } from '@angular/common/http';
import { Injectable } from '@angular/core';
import { Observable } from 'rxjs';

import { ClientStatusResponseDto } from '../dto/client-status-response.dto';
import { LoginRequestDto } from '../dto/login-request.dto';
import { LoginResponseDto } from '../dto/login-response.dto';
import { MeResponseDto } from '../dto/me-response.dto';
import { UpdateProfileImageResponseDto } from '../dto/update-profile-image-response.dto';

@Injectable({
  providedIn: 'root'
})
export class AuthApiService {
  constructor(private readonly http: HttpClient) {}

  getClientStatus(apiBaseUrl: string): Observable<ClientStatusResponseDto> {
    return this.http.get<ClientStatusResponseDto>(
      this.buildUrl(apiBaseUrl, '/api/client/status')
    );
  }

  login(
    apiBaseUrl: string,
    payload: LoginRequestDto
  ): Observable<LoginResponseDto> {
    return this.http.post<LoginResponseDto>(
      this.buildUrl(apiBaseUrl, '/api/client/auth/login'),
      payload
    );
  }

  me(apiBaseUrl: string, accessToken: string): Observable<MeResponseDto> {
    return this.http.get<MeResponseDto>(
      this.buildUrl(apiBaseUrl, '/api/client/me'),
      { headers: this.authHeaders(accessToken) }
    );
  }

  logout(apiBaseUrl: string, accessToken: string): Observable<{ message: string }> {
    return this.http.post<{ message: string }>(
      this.buildUrl(apiBaseUrl, '/api/client/auth/logout'),
      {},
      { headers: this.authHeaders(accessToken) }
    );
  }

  getProfileImage(apiBaseUrl: string, accessToken: string): Observable<Blob> {
    return this.http.get(
      this.buildUrl(apiBaseUrl, '/api/client/me/profile-image'),
      {
        headers: this.authHeaders(accessToken),
        responseType: 'blob'
      }
    );
  }

  updateProfileImage(
    apiBaseUrl: string,
    accessToken: string,
    imageFile: File
  ): Observable<UpdateProfileImageResponseDto> {
    const formData = new FormData();
    formData.append('image', imageFile);

    return this.http.post<UpdateProfileImageResponseDto>(
      this.buildUrl(apiBaseUrl, '/api/client/me/profile-image'),
      formData,
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
