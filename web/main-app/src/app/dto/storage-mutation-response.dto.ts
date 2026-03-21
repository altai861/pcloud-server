import { StorageEntryDto } from './storage-entry.dto';

export interface StorageMutationResponseDto {
  message: string;
  entry: StorageEntryDto;
}
