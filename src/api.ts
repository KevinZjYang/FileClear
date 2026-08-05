import { invoke } from "@tauri-apps/api/core";
import type {
  CleanFileResult,
  FileEntry,
  MetadataInfo,
  Settings,
} from "./types";

export function addPaths(paths: string[]): Promise<FileEntry[]> {
  return invoke<FileEntry[]>("add_paths", { paths });
}

export function readMetadata(path: string): Promise<MetadataInfo> {
  return invoke<MetadataInfo>("read_metadata", { path });
}

export function cleanFiles(paths: string[]): Promise<CleanFileResult[]> {
  return invoke<CleanFileResult[]>("clean_files", { paths });
}

export function getSettings(): Promise<Settings> {
  return invoke<Settings>("get_settings");
}

export function setContextMenuEnabled(enabled: boolean): Promise<void> {
  return invoke<void>("set_context_menu_enabled", { enabled });
}

export function isContextMenuRegistered(): Promise<boolean> {
  return invoke<boolean>("is_context_menu_registered");
}

export function openInExplorer(path: string): Promise<void> {
  return invoke<void>("open_in_explorer", { path });
}

export const SUPPORTED_EXTENSIONS =
  "*.jpg;*.jpeg;*.png;*.gif;*.webp;*.tiff;*.tif;*.bmp;*.pdf;*.docx;*.doc;*.xlsx;*.xls;*.pptx;*.ppt";