import { Injectable } from '@angular/core';
import { BehaviorSubject } from 'rxjs';

@Injectable({
  providedIn: 'root'
})
export class ProfileImageService {
  private readonly srcSubject = new BehaviorSubject<string | null>(null);
  readonly profileImageSrc$ = this.srcSubject.asObservable();

  private currentObjectUrl: string | null = null;

  setFromBlob(blob: Blob): void {
    this.clearObjectUrl();
    this.currentObjectUrl = URL.createObjectURL(blob);
    this.srcSubject.next(this.currentObjectUrl);
  }

  clear(): void {
    this.clearObjectUrl();
    this.srcSubject.next(null);
  }

  private clearObjectUrl(): void {
    if (this.currentObjectUrl) {
      URL.revokeObjectURL(this.currentObjectUrl);
      this.currentObjectUrl = null;
    }
  }
}
