export interface AppInfo {
  name: string;
  version: string;
  build_status: string;
  environment: string;
}

export interface SafeErrorResponse {
  code: string;
  message: string;
}

/**
 * Invokes the backend `get_app_info` command.
 * Gracefully handles standalone browser environments when Tauri IPC is not available.
 */
export async function getAppInfo(): Promise<AppInfo> {
  try {
    // Check if running inside Tauri desktop webview
    if (typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window) {
      const { invoke } = await import('@tauri-apps/api/core');
      return await invoke<AppInfo>('get_app_info');
    }
  } catch (err) {
    console.warn('Tauri IPC invoke error, falling back to local metadata:', err);
  }

  // Graceful fallback for web preview / development mode
  return {
    name: 'LocardX',
    version: '0.1.0',
    build_status: 'Foundation Verified',
    environment: 'development (web preview)',
  };
}
