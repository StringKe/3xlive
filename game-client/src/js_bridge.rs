use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    // ─── auth 桥接 ───

    #[wasm_bindgen(js_namespace = __auth, js_name = requestTicket)]
    pub async fn request_ticket() -> JsValue;

    #[wasm_bindgen(js_namespace = __auth, js_name = openSignIn)]
    pub fn open_sign_in();

    #[wasm_bindgen(js_namespace = __auth, js_name = openUserProfile)]
    pub fn open_user_profile();

    #[wasm_bindgen(js_namespace = __auth, js_name = getAuthState)]
    pub fn get_auth_state() -> JsValue;

    #[wasm_bindgen(js_namespace = __auth, js_name = signOut)]
    pub async fn sign_out();

    #[wasm_bindgen(js_namespace = __auth, js_name = stateVersion)]
    pub fn auth_state_version() -> u32;

    // ─── i18n 桥接 ───

    #[wasm_bindgen(js_namespace = __i18n)]
    pub fn locale() -> String;

    #[wasm_bindgen(js_namespace = __i18n, js_name = fontPath)]
    pub fn font_path() -> String;

    #[wasm_bindgen(js_namespace = __i18n)]
    pub fn t(key: &str) -> String;

    #[wasm_bindgen(js_namespace = __i18n, js_name = setLocale)]
    pub fn set_locale(locale: &str);

    // ─── storage 桥接 ───

    #[wasm_bindgen(js_namespace = __storage, js_name = get)]
    pub async fn storage_get(store: &str, key: &str) -> JsValue;

    #[wasm_bindgen(js_namespace = __storage, js_name = put)]
    pub async fn storage_put(store: &str, key: &str, value: &[u8]);

    #[wasm_bindgen(js_namespace = __storage, js_name = del)]
    pub async fn storage_del(store: &str, key: &str);

    #[wasm_bindgen(js_namespace = __storage, js_name = getAll)]
    pub async fn storage_get_all(store: &str) -> JsValue;

    #[wasm_bindgen(js_namespace = __storage, js_name = getAllKeys)]
    pub async fn storage_get_all_keys(store: &str) -> JsValue;

    #[wasm_bindgen(js_namespace = __storage, js_name = clear)]
    pub async fn storage_clear(store: &str);

    // ─── sync 桥接（HTTP 同步） ───

    #[wasm_bindgen(js_namespace = __sync, js_name = uploadResults)]
    pub async fn sync_upload_results(data: &[u8]) -> JsValue;

    #[wasm_bindgen(js_namespace = __sync, js_name = pullState)]
    pub async fn sync_pull_state() -> JsValue;

    #[wasm_bindgen(js_namespace = __sync, js_name = submitArena)]
    pub async fn sync_submit_arena(replay: &[u8]) -> JsValue;

    #[wasm_bindgen(js_namespace = __sync, js_name = getSeasonInfo)]
    pub async fn sync_get_season_info() -> JsValue;

    #[wasm_bindgen(js_namespace = __sync, js_name = sendMail)]
    pub async fn sync_send_mail(data: &[u8]);

    #[wasm_bindgen(js_namespace = __sync, js_name = claimMail)]
    pub async fn sync_claim_mail(mail_id: f64) -> JsValue;
}

/// requestTicket() 返回的票据信息
/// 成功时包含 playerId/displayName/ticket，失败时包含 error
#[derive(serde::Deserialize, Default)]
pub struct TicketResponse {
    #[serde(rename = "playerId")]
    pub player_id: Option<String>,
    #[serde(rename = "displayName")]
    pub display_name: Option<String>,
    pub ticket: Option<String>,
    pub error: Option<String>,
}

/// getAuthState() 返回的认证状态
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthState {
    pub is_clerk_signed_in: bool,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
}
