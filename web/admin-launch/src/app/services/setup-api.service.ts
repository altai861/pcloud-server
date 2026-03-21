import { HttpClient } from '@angular/common/http';
import { Injectable } from '@angular/core';
import { Observable } from 'rxjs';

import { SetupInitializeRequestDto } from '../dto/setup-initialize-request.dto';
import { SetupInitializeResponseDto } from '../dto/setup-initialize-response.dto';
import { SetupStatusResponseDto } from '../dto/setup-status-response.dto';

@Injectable({
  providedIn: 'root'
})
export class SetupApiService {
  constructor(private readonly http: HttpClient) {}

  getSetupStatus(): Observable<SetupStatusResponseDto> {
    return this.http.get<SetupStatusResponseDto>('/api/setup/status');
  }

  initializeSetup(
    payload: SetupInitializeRequestDto
  ): Observable<SetupInitializeResponseDto> {
    return this.http.post<SetupInitializeResponseDto>(
      '/api/setup/initialize',
      payload
    );
  }
}
