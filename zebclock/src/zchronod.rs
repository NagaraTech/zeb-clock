use crate::vlc::ClockInfo;
use crate::{node_factory::ZchronodFactory, storage::Storage, vlc::Clock};
use node_api::config::ZchronodConfig;
use protos::zmessage::ZMessage;
use tools::helper::sha256_str_to_hex;
use std::collections::{BTreeMap, VecDeque};
use std::{cmp, sync::Arc};
use tokio::net::UdpSocket;
use tokio::sync::RwLock;
use tracing::*;

pub struct Zchronod {
    pub config: Arc<ZchronodConfig>,
    pub socket: UdpSocket,
    pub storage: Storage,
    pub state: RwLock<ServerState>,
}

pub type ZchronodArc = Arc<Zchronod>;

impl Zchronod {
    pub fn zchronod_factory() -> ZchronodFactory {
        ZchronodFactory::init()
    }
}

/// A cache state of a server node.
#[derive(Debug, Clone)]
pub struct ServerState {
    pub clock_info: ClockInfo,
    pub message_ids: VecDeque<String>,
    pub cache_items: BTreeMap<String, ZMessage>,
    pub cache_maximum: u64,
}

impl ServerState {
    /// Create a new server state.
    pub fn new(node_id: String, cache_maximum: u64) -> Self {
        Self {
            clock_info: ClockInfo::new(
                Clock::new(),
                String::new(),
                node_id.clone(),
                "".to_owned(),
                0,
            ),
            message_ids: VecDeque::new(),
            cache_items: BTreeMap::new(),
            cache_maximum,
        }
    }

    /// Add items into the state. Returns true if resulting in a new state.
    pub fn add(&mut self, items: Vec<ZMessage>) -> bool {
        if items.is_empty() {
            return false;
        } 

        for item in items.iter() {
            // filter replicate message id
            let msg_id = hex::encode(item.id.clone());
            if !self.cache_items.contains_key(&msg_id)  {
                if self.message_ids.len() > self.cache_maximum.try_into().unwrap() {
                    let old_id = self.message_ids.pop_front().unwrap_or(String::new());
                    self.cache_items.remove(&old_id);
                }
                self.message_ids.push_back(msg_id.clone());
                self.cache_items.insert(msg_id, item.clone());
            } else {
                info!("duplicate message_id, skip & no action");
                continue;
            }
        }

        let now = tools::helper::get_time_ms();
        self.clock_info.clock.inc(self.clock_info.node_id.clone());
        self.clock_info.count += 1;
        self.clock_info.create_at = now;
        if let Some(last) = items.last() {
            let last_id = hex::encode(last.id.clone());
            self.clock_info.message_id = last_id;

            let clock_str = serde_json::to_string(&self.clock_info.clock).unwrap();
            let hash_hex = sha256_str_to_hex(clock_str.clone());
            self.clock_info.clock_hash = hash_hex;
        }

        true
    }

    /// Merge another ServerState into the current state. Returns true if
    /// resulting in a new state (different from current and received
    /// state).
    pub fn merge(&mut self, from_clock: ClockInfo, items: &Vec<ZMessage>) -> (bool, bool) {
        match self.clock_info.clock.partial_cmp(&from_clock.clock) {
            Some(cmp::Ordering::Equal) => (false, false),
            Some(cmp::Ordering::Greater) => (false, false),
            Some(cmp::Ordering::Less) | None => {
                // todo: can merge when just one event last
                self.clock_info.clock.merge(&vec![&from_clock.clock]);
                let added = self.add(items.to_vec());
                (added, added)
            }
        }
    }
}