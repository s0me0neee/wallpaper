export interface AppConfig {
  image_dir: string | null;
  order: string;
  number_of_cols: number;
  subdir: boolean;
}

export interface ImageEntry {
  index: number;
  path: string;
  thumbnail: string;
}

export interface LoadDone {
  loaded: number;
  skipped: number;
}
