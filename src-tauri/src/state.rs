//! Uygulama geneli durum (PLAN.md §2.1).

use std::sync::Arc;

use crate::db::DbPool;
use crate::discovery::DiscoveryService;
use crate::identity::Identity;
use crate::network::manager::ConnectionManager;
use crate::network::service::NetworkService;
use crate::pairing::PairingManager;
use crate::paths::AppPaths;

pub struct AppState {
    pub paths: AppPaths,
    pub db: DbPool,
    pub identity: Identity,
    pub network: NetworkService,
    pub discovery: DiscoveryService,
    pub pairing: Arc<PairingManager>,
    pub connections: Arc<ConnectionManager>,
}
