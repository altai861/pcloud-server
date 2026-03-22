import { CommonModule } from '@angular/common';
import { Component, OnDestroy, OnInit } from '@angular/core';
import { RouterLink } from '@angular/router';
import { Subscription } from 'rxjs';

import { RecentStorageEntryModel } from '../../models/recent-storage-entry.model';
import { TPipe } from '../../pipes/t.pipe';
import { RecentEntriesService } from '../../services/recent-entries.service';

@Component({
  selector: 'app-home-page',
  imports: [CommonModule, RouterLink, TPipe],
  templateUrl: './home-page.component.html',
  styleUrl: './home-page.component.css'
})
export class HomePageComponent implements OnInit, OnDestroy {
  private recentEntriesSub: Subscription | null = null;
  recentEntries: RecentStorageEntryModel[] = [];

  constructor(private readonly recentEntriesService: RecentEntriesService) {}

  ngOnInit(): void {
    this.recentEntriesSub = this.recentEntriesService.recentEntries$.subscribe((entries) => {
      this.recentEntries = entries;
    });
  }

  ngOnDestroy(): void {
    this.recentEntriesSub?.unsubscribe();
    this.recentEntriesSub = null;
  }

  formatSize(bytes: number | null): string {
    if (bytes === null || !Number.isFinite(bytes) || bytes < 0) {
      return '-';
    }

    const units = ['B', 'KB', 'MB', 'GB', 'TB', 'PB'];
    let value = bytes;
    let index = 0;

    while (value >= 1000 && index < units.length - 1) {
      value /= 1000;
      index += 1;
    }

    if (index === 0) {
      return `${Math.round(value)} ${units[index]}`;
    }

    const rounded = value >= 100 ? value.toFixed(0) : value >= 10 ? value.toFixed(1) : value.toFixed(2);
    const compact = rounded.replace(/\.0+$/, '').replace(/(\.\d*[1-9])0+$/, '$1');

    return `${compact} ${units[index]}`;
  }

  formatDate(unixMillis: number | null): string {
    if (unixMillis === null || !Number.isFinite(unixMillis)) {
      return '-';
    }

    const date = new Date(unixMillis);
    if (Number.isNaN(date.getTime())) {
      return '-';
    }

    return new Intl.DateTimeFormat(undefined, {
      year: 'numeric',
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit'
    }).format(date);
  }
}
