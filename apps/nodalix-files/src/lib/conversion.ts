import { invoke } from "@tauri-apps/api/core";

export interface ConversionCapability {
  from_ext: string;
  to_ext: string;
  label: string;
  backend: string;
  lossless: boolean;
  lossy: boolean;
  requires_external: boolean;
  available: boolean;
  unavailable_reason: string | null;
  warnings: string[];
  user_note: string | null;
}

export type RenameConversionScenario =
  | "no_extension_change"
  | "convert_available"
  | "incompatible"
  | "unavailable"
  | "category_unavailable";

export interface RenameConversionPreview {
  old_path: string;
  old_name: string;
  new_name: string;
  dest_path: string;
  dest_exists: boolean;
  extension_changed: boolean;
  from_ext: string | null;
  to_ext: string | null;
  scenario: RenameConversionScenario;
  capability: ConversionCapability | null;
  message: string | null;
  ask_on_extension_change: boolean;
  keep_original: boolean;
}

export interface ConversionSettings {
  ask_on_extension_change: boolean;
  keep_original: boolean;
  allow_external_tools: boolean;
  tools_available: Record<string, boolean>;
}

export const previewRenameWithConversion = (oldPath: string, newName: string) =>
  invoke<RenameConversionPreview>("preview_rename_with_conversion", {
    oldPath,
    newName,
  });

export const convertFileToPath = (sourcePath: string, destPath: string) =>
  invoke<string>("convert_file_to_path", { sourcePath, destPath });

export const getFileConversionSettings = () =>
  invoke<ConversionSettings>("get_file_conversion_settings");
