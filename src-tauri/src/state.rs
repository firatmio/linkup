//! Uygulama geneli durum (PLAN.md §2.1).

use crate::db::DbPool;
use crate::discovery::DiscoveryService;
use crate::identity::Identity;
use crate::network::service::NetworkService;
use crate::paths::AppPaths;

pub struct AppState {
    pub paths: AppPaths,
    pub db: DbPool,
    pub identity: Identity,
    /// Uygulama kapanırken uç noktanın da kapanması için burada tutulur;
    /// okunması gerekmiyor, yaşaması gerekiyor.
    #[allow(dead_code)]
    pub network: NetworkService,
    pub discovery: DiscoveryService,
}

impl AppState {
    pub fn new(
        paths: AppPaths,
        db: DbPool,
        identity: Identity,
        network: NetworkService,
        discovery: DiscoveryService,
    ) -> Self {
        Self {
            paths,
            db,
            identity,
            network,
            discovery,
        }
    }
}
