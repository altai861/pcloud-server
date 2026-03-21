import { Injectable } from '@angular/core';
import { BehaviorSubject } from 'rxjs';

import { StorageEntryDto } from '../dto/storage-entry.dto';
import { RecentStorageEntryModel } from '../models/recent-storage-entry.model';

@Injectable({
  providedIn: 'root'
})
export class RecentEntriesService {
  private readonly storageKey = 'pcloud.recentEntries';
  private readonly maxEntries = 40;

  private readonly recentEntriesSubject = new BehaviorSubject<RecentStorageEntryModel[]>(
    this.readFromStorage()
  );

  readonly recentEntries$ = this.recentEntriesSubject.asObservable();

  recordOpened(entry: StorageEntryDto): void {
    const nextItem: RecentStorageEntryModel = {
      ...entry,
      openedAtUnixMs: Date.now()
    };

    const deduplicated = this.recentEntriesSubject.value.filter(
      (current) => current.path !== entry.path || current.entryType !== entry.entryType
    );

    const nextList = [nextItem, ...deduplicated].slice(0, this.maxEntries);
    this.recentEntriesSubject.next(nextList);
    this.persist(nextList);
  }

  private persist(entries: RecentStorageEntryModel[]): void {
    try {
      localStorage.setItem(this.storageKey, JSON.stringify(entries));
    } catch {
      // Ignore storage errors and keep runtime behavior.
    }
  }

  private readFromStorage(): RecentStorageEntryModel[] {
    try {
      const raw = localStorage.getItem(this.storageKey);
      if (!raw) {
        return [];
      }

      const parsed = JSON.parse(raw) as unknown;
      if (!Array.isArray(parsed)) {
        return [];
      }

      return parsed
        .filter((item): item is RecentStorageEntryModel => {
          return (
            item !== null &&
            typeof item === 'object' &&
            typeof (item as RecentStorageEntryModel).name === 'string' &&
            typeof (item as RecentStorageEntryModel).path === 'string' &&
            ((item as RecentStorageEntryModel).entryType === 'folder' ||
              (item as RecentStorageEntryModel).entryType === 'file') &&
            (typeof (item as RecentStorageEntryModel).openedAtUnixMs === 'number' ||
              typeof (item as RecentStorageEntryModel).openedAtUnixMs === 'string')
          );
        })
        .map((item) => ({
          ...item,
          openedAtUnixMs: Number(item.openedAtUnixMs)
        }))
        .filter((item) => Number.isFinite(item.openedAtUnixMs))
        .slice(0, this.maxEntries);
    } catch {
      return [];
    }
  }
}
