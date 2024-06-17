//! Verifiable logical clock.
//!
//! Please reference create/vlc package. Update contents: 
//! * 1. Turn the index id to string type.
//! * 2. New add the clockinfo & mergelog object.

use serde::{Deserialize, Serialize};
use std::cmp;
use std::collections::HashMap;
use db_sql::pg::entities::clock_infos::Model as ClockInfoModel;
use db_sql::pg::entities::merge_logs::Model as MergeLogModel;
use protos::vlc::ClockInfo as ProtoClockInfo;

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug, Default)]
pub struct Clock {
    pub values: HashMap<String, u128>,
}

impl PartialOrd for Clock {
    fn partial_cmp(&self, other: &Clock) -> Option<cmp::Ordering> {
        let mut less = false;
        let mut greater = false;

        for (id, value) in &self.values {
            let other_value = other.values.get(id);
            if other_value.is_none() || value > other_value.unwrap() {
                greater = true;
            } else if value < other_value.unwrap() {
                less = true;
            }
        }

        for (id, _) in &other.values {
            if self.values.get(id).is_none() {
                less = true;
            }
        }

        if less && greater {
            None
        } else if less {
            Some(cmp::Ordering::Less)
        } else if greater {
            Some(cmp::Ordering::Greater)
        } else {
            Some(cmp::Ordering::Equal)
        }
    }
}

impl Clock {
    /// Create a new clock.
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
        }
    }

    /// Increment the clock
    pub fn inc(&mut self, id: String) {
        let value = self.values.entry(id).or_insert(0);
        *value += 1;
    }

    /// Get the clock count by id
    pub fn get(&mut self, id: String) -> u128 {
        let value = self.values.entry(id).or_insert(0);
        *value
    }

    /// Reset the clock.
    pub fn clear(&mut self) {
        self.values.clear();
    }

    /// Merge the clock with other clocks.
    pub fn merge(&mut self, others: &Vec<&Clock>) {
        for &clock in others {
            for (id, value) in &clock.values {
                let v = self.values.entry(id.clone()).or_insert(0);
                *v = std::cmp::max(*v, *value);
            }
        }
    }

    /// Diff is local clock minus another clock
    pub fn diff(&self, other: &Clock) -> Clock {
        let mut ret = Clock::new();
        for (id, v1) in &self.values {
            let v2 = other.values.get(id).unwrap_or(&0);
            if v1 > v2 {
                ret.values.insert(id.clone(), v1-v2);
            } else {
                ret.values.insert(id.clone(), 0);
            }
        }
        ret
    }

    /// return index key of clock
    pub fn index_key(&self) -> String {
        let mut key: String = String::new();
        for (index, value) in &self.values {
            key = format!("{}{}-{}-", key, index, value);
        }
        key
    }

    /// return common base clock of two clock
    pub fn base_common(&self, other: &Clock) -> Clock {
        let mut ret = Clock::new();
        for (id, v1) in &self.values {
            let v2 = other.values.get(id).unwrap_or(&0);
            if v1 <= v2 {
                ret.values.insert(id.clone(), *v1);
            } else {
                ret.values.insert(id.clone(), *v2);
            }
        }
        ret
    }

    /// return true when all value is zero in clock dimensions
    pub fn is_genesis(&self) -> bool {
        let sum: u128 = self.values.values().sum();
        sum == 0
    }
    
}

/// Clock info sinker to db.
/// id is server node id, count is the event count in this server.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ClockInfo {
    pub clock: Clock,
    pub clock_hash: String,
    pub node_id: String,  
    pub message_id: String,
    pub count: u128,
    pub create_at: u128,
}

impl ClockInfo {
    pub fn new(clock: Clock, clock_hash: String,node_id: String, message_id: String, count: u128) -> Self {
        let create_at = tools::helper::get_time_ms();
        Self { clock, clock_hash, node_id, message_id, count, create_at }
    }
}

impl From<&ProtoClockInfo> for ClockInfo {
    fn from(protobuf_clock_info: &ProtoClockInfo) -> Self {
        let clock = protobuf_clock_info
            .clock
            .as_ref()
            .map(|c| {
                Clock {
                    values: c
                        .values
                        .iter()
                        .map(|(k, v)| (k.clone(), *v as u128))
                        .collect(),
                }
            }).unwrap();
        
        let clock_hash_hex = hex::encode(&protobuf_clock_info.clock_hash);
        let node_id = hex::encode(&protobuf_clock_info.node_id);
        let message_id = hex::encode(&protobuf_clock_info.message_id);
        let count = protobuf_clock_info.count;
        let create_at = protobuf_clock_info.create_at;

        ClockInfo {
            clock,
            clock_hash: clock_hash_hex,
            node_id,
            message_id,
            count: count.into(),
            create_at: create_at.into(),
        }
    }
}

impl From<ClockInfoModel> for ClockInfo {
    fn from(model: ClockInfoModel) -> Self {
        let clock: Clock = serde_json::from_str(&model.clock).unwrap_or_else(|_| Clock::default());
        let create_at = model.create_at.map(|dt| dt.and_utc().timestamp_millis() as u128).unwrap_or(0);

        ClockInfo {
            clock,
            clock_hash: model.clock_hash,
            node_id: model.node_id,
            message_id: model.message_id,
            count: model.event_count as u128,
            create_at,
        }
    }
}

/// MergeLog sinker to db.
/// id is server node id, count is the event count in this server.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MergeLog {
    pub from_id: String,
    pub to_id: String,    // to_node trigger merge action
    pub start_count: u128,
    pub end_count: u128,
    pub s_clock_hash: String,
    pub e_clock_hash: String,
    pub merge_at: u128,
}

impl From<MergeLogModel> for MergeLog {
    fn from(model: MergeLogModel) -> Self {
        let merge_at = model.merge_at.and_utc().timestamp_millis() as u128;

        MergeLog {
            from_id: model.from_id,
            to_id: model.to_id,
            start_count: model.start_count as u128,
            end_count: model.end_count as u128,
            s_clock_hash: model.s_clock_hash,
            e_clock_hash: model.e_clock_hash,
            merge_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clock_inc() {
        let mut c = Clock::new();
        c.inc("0".to_owned());
        c.inc("0".to_owned());
        assert_eq!(c.values.get(&"0".to_owned()), Some(&2));
    }

    #[test]
    fn clock_cmp() {
        let mut c1 = Clock::new();
        c1.inc("0".to_owned());
        let c2 = c1.clone();
        let mut c3 = Clock::new();
        c3.inc("1".to_owned());

        assert_eq!(c1, c2);
        assert_eq!(c1.partial_cmp(&c3), None);
        assert_eq!(c2.partial_cmp(&c3), None);

        c1.inc("0".to_owned());
        assert_eq!(c2.partial_cmp(&c1), Some(cmp::Ordering::Less));
        assert_eq!(c3.partial_cmp(&c1), None);
    }

    #[test]
    fn clock_merge() {
        let mut c1 = Clock::new();
        c1.inc("0".to_owned());
        let mut c2 = Clock::new();
        c2.inc("1".to_owned());
        let mut c3 = Clock::new();
        c3.inc("2".to_owned());

        assert_eq!(c1.partial_cmp(&c2), None);
        assert_eq!(c1.partial_cmp(&c3), None);
        assert_eq!(c2.partial_cmp(&c3), None);

        c1.merge(&vec![&c2, &c3]);
        assert_eq!(c2.partial_cmp(&c1), Some(cmp::Ordering::Less));
        assert_eq!(c1.partial_cmp(&c2), Some(cmp::Ordering::Greater));
        assert_eq!(c3.partial_cmp(&c1), Some(cmp::Ordering::Less));
        assert_eq!(c1.partial_cmp(&c3), Some(cmp::Ordering::Greater));
    }
}
