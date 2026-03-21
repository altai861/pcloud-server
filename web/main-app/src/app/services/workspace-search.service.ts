import { Injectable } from '@angular/core';
import { BehaviorSubject } from 'rxjs';

@Injectable({
  providedIn: 'root'
})
export class WorkspaceSearchService {
  private readonly termSubject = new BehaviorSubject<string>('');
  readonly searchTerm$ = this.termSubject.asObservable();

  setSearchTerm(value: string): void {
    this.termSubject.next(value);
  }
}
