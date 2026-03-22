import { SearchResourceEntryDto } from './search-resource-entry.dto';

export interface SearchResourcesResponseDto {
  query: string;
  entries: SearchResourceEntryDto[];
  nextCursor: string | null;
  hasMore: boolean;
}
