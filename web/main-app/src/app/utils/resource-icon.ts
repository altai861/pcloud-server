type ResourceKind = 'folder' | 'file';

const ICON_BASE = 'google-icons/';

const IMAGE_EXTENSIONS = new Set([
  'jpg', 'jpeg', 'png', 'gif', 'webp', 'bmp', 'svg', 'heic', 'heif', 'tif', 'tiff', 'ico', 'avif'
]);
const DOCUMENT_EXTENSIONS = new Set([
  'txt', 'md', 'rtf', 'doc', 'docx', 'odt', 'pages', 'epub', 'xls', 'xlsx', 'ods', 'csv', 'tsv',
  'numbers', 'ppt', 'pptx', 'odp', 'key'
]);
const ARCHIVE_EXTENSIONS = new Set(['zip', 'rar', '7z', 'tar', 'gz', 'bz2', 'xz', 'tgz']);
const AUDIO_EXTENSIONS = new Set(['mp3', 'wav', 'flac', 'ogg', 'aac', 'm4a', 'wma']);
const VIDEO_EXTENSIONS = new Set(['mp4', 'mov', 'mkv', 'webm', 'avi', 'wmv', 'flv', 'm4v']);

export function resourceIconSrc(resourceType: ResourceKind, name: string): string {
  if (resourceType === 'folder') {
    return `${ICON_BASE}folder.svg`;
  }

  const extension = fileExtension(name);

  if (extension === 'pdf') {
    return `${ICON_BASE}pdf_icon.svg`;
  }

  if (IMAGE_EXTENSIONS.has(extension)) {
    return `${ICON_BASE}image.svg`;
  }

  if (ARCHIVE_EXTENSIONS.has(extension)) {
    return `${ICON_BASE}folder_zip.svg`;
  }

  if (AUDIO_EXTENSIONS.has(extension)) {
    return `${ICON_BASE}audio_file.svg`;
  }

  if (VIDEO_EXTENSIONS.has(extension)) {
    return `${ICON_BASE}video_file.svg`;
  }

  if (DOCUMENT_EXTENSIONS.has(extension)) {
    return `${ICON_BASE}docs.svg`;
  }

  return `${ICON_BASE}file.svg`;
}

function fileExtension(name: string): string {
  const trimmed = name.trim().toLowerCase();
  const lastDotIndex = trimmed.lastIndexOf('.');

  if (lastDotIndex <= 0 || lastDotIndex === trimmed.length - 1) {
    return '';
  }

  return trimmed.slice(lastDotIndex + 1);
}
