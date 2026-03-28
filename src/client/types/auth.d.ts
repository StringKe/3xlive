// window.__auth 类型声明
export interface AuthState {
  isClerkSignedIn: boolean;
  displayName: string;
  avatarUrl: string | null;
  stateVersion: number;
}

export interface AuthBridge {
  stateVersion(): number;
  requestTicket(): Promise<unknown>;
  openSignIn(): void;
  openUserProfile(): void;
  getAuthState(): AuthState;
  signOut(): Promise<void>;
}

export interface I18nBridge {
  locale(): string;
  fontPath(): string;
  t(key: string): string;
  setLocale(locale: string): void;
}

declare global {
  interface Window {
    __auth: AuthBridge;
    __i18n: I18nBridge;
  }
}
