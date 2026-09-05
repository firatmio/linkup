//! Uygulama geneli durum. İleriki fazlarda ağ endpoint'i ve bağlantı
//! yöneticisi buraya eklenecek (PLAN.md §2.1).

use crate::db::DbPool;
use crate::identity::Identity;
use crate::paths::AppPaths;

pub struct AppState {
    pub paths: AppPaths,
    pub db: DbPool,
    pub identity: Identity,
}

impl AppState {
    pub fn new(paths: AppPaths, db: DbPool, identity: Identity) -> Self {
        Self {
            paths,
            db,
            identity,
        }
    }
}
