import { Pipe, PipeTransform } from '@angular/core';

import { I18nService } from '../services/i18n.service';

@Pipe({
  name: 't',
  standalone: true,
  pure: false
})
export class TPipe implements PipeTransform {
  constructor(private readonly i18n: I18nService) {}

  transform(key: string | null | undefined, params?: Record<string, string | number | null | undefined>): string {
    if (!key) {
      return '';
    }

    return this.i18n.t(key, params);
  }
}
