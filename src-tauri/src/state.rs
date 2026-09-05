//! Uygulama geneli durum. İleriki fazlarda DB havuzu, ağ endpoint'i ve
//! bağlantı yöneticisi buraya eklenecek (PLAN.md §2.1).

use crate::paths::AppPaths;

pub struct AppState {
    pub paths: AppPaths,
}

impl AppState {
    pub fn new(paths: AppPaths) -> Self {
        Self { paths }
    }
}
