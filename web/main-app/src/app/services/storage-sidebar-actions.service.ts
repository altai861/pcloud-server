import { Injectable } from '@angular/core';
import { Observable, Subject } from 'rxjs';

export type StorageSidebarAction =
  | { type: 'create-folder' }
  | { type: 'upload-file'; file: File };

@Injectable({
  providedIn: 'root'
})
export class StorageSidebarActionsService {
  private readonly actionSubject = new Subject<StorageSidebarAction>();
  private readonly queuedActions: StorageSidebarAction[] = [];

  readonly actions$: Observable<StorageSidebarAction> = this.actionSubject.asObservable();

  emit(action: StorageSidebarAction): void {
    this.actionSubject.next(action);
  }

  queue(action: StorageSidebarAction): void {
    this.queuedActions.push(action);
  }

  consumeQueued(): StorageSidebarAction[] {
    if (this.queuedActions.length === 0) {
      return [];
    }

    const snapshot = [...this.queuedActions];
    this.queuedActions.length = 0;

    return snapshot;
  }
}
