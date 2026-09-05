//! Uygulama geneli durum (PLAN.md §2.1).

use crate::db::DbPool;
use crate::identity::Identity;
use crate::network::service::NetworkService;
use crate::paths::AppPaths;

pub struct AppState {
    pub paths: AppPaths,
    pub db: DbPool,
    pub identity: Identity,
    /// Uygulama kapanırken uç noktanın da kapanması için burada tutulur;
    /// okunması gerekmiyor, yaşaması gerekiyor. Faz 3'te keşif ve bağlanma
    /// komutları buradan geçecek.
    #[allow(dead_code)]
    pub network: NetworkService,
}

impl AppState {
    pub fn new(paths: AppPaths, db: DbPool, identity: Identity, network: NetworkService) -> Self {
        Self {
            paths,
            db,
            identity,
            network,
        }
    }
}
