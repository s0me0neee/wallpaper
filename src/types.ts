export interface PostCommand {
  cmds: string[];
}

export interface AppConfig {
  image_dir: string | null;
  order: string;
  number_of_cols: number;
  subdir: boolean;
  window_width: number;
  window_height: number;
  post_command: PostCommand;
  skip_set_wallpaper: boolean;
}

export interface ImageEntry {
  index: number;
  path: string;
  thumbnail: string;
  modified: number;
  size: number;
}

export interface LoadDone {
  loaded: number;
  skipped: number;
}
