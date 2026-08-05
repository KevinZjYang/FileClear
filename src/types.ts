export interface FileEntry {
  path: string;
  name: string;
  size: number;
  fileType: string;
  modified: number;
  supported: boolean;
}

export interface MetadataField {
  key: string;
  value: string;
}

export interface MetadataInfo {
  path: string;
  fileType: string;
  fields: MetadataField[];
  warnings: string[];
}

export interface CleanFileResult {
  path: string;
  success: boolean;
  error: string | null;
  warnings: string[];
  originalSize: number;
  cleanedSize: number;
}

export interface ProgressEvent {
  current: number;
  total: number;
  name: string;
}

export interface QuickCleanFinished {
  success: number;
  failed: number;
  firstError: string;
}

export interface Settings {
  contextMenuEnabled: boolean;
}

export type FileStatus = "pending" | "cleaning" | "success" | "failed";

export interface TableRow extends FileEntry {
  status: FileStatus;
  message: string;
}