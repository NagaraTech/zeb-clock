use std::{sync::Arc, time::Duration};
use chrono::{Local, NaiveDateTime};
use db_sql::pg::entities::{merge_logs, z_messages};
use node_api::config::ZchronodConfig;
// use db_sql::api::{DbKindZchronod, DbWrite};
use db_sql::pg::entities::{clock_infos, prelude::{ClockInfos, MergeLogs, ZMessages}};
use protos::zmessage::ZMessage as ProtoZMessage;
use sea_orm::*;
use crate::vlc::ClockInfo;
use crate::vlc::MergeLog;
use tracing::{error, info};

pub struct Storage {
    // pub zchronod_db: DbWrite<DbKindZchronod>,
    pub pg_db: Arc<DatabaseConnection>
}

impl Storage {
    pub async fn new(config: Arc<ZchronodConfig>) -> Self {
        // let zchronod_db = DbWrite::open(
        //     config.storage_root_path.as_ref().unwrap().as_path(),
        //     DbKindZchronod,
        // ).unwrap(); // todo error handling
        
        // connect to pg db
        let url = format!("{}/{}", config.db.pg_db_url, config.db.pg_db_name);
        let mut opt = ConnectOptions::new(&url);
        opt.max_connections(config.db.max_connect_pool)
            .min_connections(config.db.min_connect_pool)
            .connect_timeout(Duration::from_secs(config.db.connect_timeout)) 
            .acquire_timeout(Duration::from_secs(config.db.acquire_timeout));
        
        let pg_db = Database::connect(opt.clone()).await.expect("failed to connect to database");
        info!("max_connections={:?},connect timeout={:?}, acquire timeout={:?}", opt.get_max_connections().unwrap(), opt.get_connect_timeout().unwrap(),opt.get_acquire_timeout().unwrap());
        let pg_db_arc = Arc::new(pg_db);
        Self {
            // zchronod_db,
            pg_db: pg_db_arc,
        }
    }
    
    // postgre inner api
    pub async fn sinker_clock(&self, message_id: String, raw_message: Vec<u8>, clock_info: &ClockInfo) {
        let clock_str = serde_json::to_string(&clock_info.clock).unwrap();
        let naive_datetime = NaiveDateTime::from_timestamp_millis(clock_info.create_at.try_into().unwrap());
        let clock_info = clock_infos::ActiveModel {
            clock: ActiveValue::Set(clock_str.clone()),
            clock_hash: ActiveValue::Set(clock_info.clock_hash.clone()),
            node_id: ActiveValue::Set(clock_info.node_id.to_owned()),
            message_id: ActiveValue::Set(message_id),
            raw_message: ActiveValue::Set(raw_message),
            event_count: ActiveValue::Set(clock_info.count.try_into().unwrap()),
            create_at: ActiveValue::Set(naive_datetime),
            ..Default::default()
        };
        let res = ClockInfos::insert(clock_info).exec(self.pg_db.as_ref()).await;
        if let Err(err) = res {
            error!("Insert clock_info error, err: {}", err);
        }
    }

    pub async fn sinker_merge_log(&self, fclock_info: &ClockInfo, tclock_info: &ClockInfo) {
        let f_hash_hex = fclock_info.clock_hash.clone();
        let e_hash_hex = tclock_info.clock_hash.clone();
        let now = Local::now().timestamp_millis();
        let naive_datetime = NaiveDateTime::from_timestamp_millis(now).unwrap();
        let merge_log = merge_logs::ActiveModel {
            from_id: ActiveValue::Set(fclock_info.node_id.to_owned()),
            to_id: ActiveValue::Set(tclock_info.node_id.to_owned()),
            start_count: ActiveValue::Set(fclock_info.count.to_owned().try_into().unwrap()),
            end_count: ActiveValue::Set(tclock_info.count.to_owned().try_into().unwrap()),
            s_clock_hash: ActiveValue::Set(f_hash_hex),
            e_clock_hash: ActiveValue::Set(e_hash_hex),
            merge_at: ActiveValue::Set(naive_datetime),
            ..Default::default()
        };
        let res = MergeLogs::insert(merge_log).exec(self.pg_db.as_ref()).await;
        if let Err(err) = res {
            error!("Insert merge_log error, err: {}", err);
        }
    }

    pub async fn sinker_zmessage(&self, zmessage: ProtoZMessage) {
        let msg_id = hex::encode(zmessage.id);
        let pub_key_hex = hex::encode(zmessage.public_key);
        let from_hex = hex::encode(zmessage.from);
        let to_hex = hex::encode(zmessage.to);
        let zmessage = z_messages::ActiveModel {
            message_id: ActiveValue::Set(msg_id),
            version: ActiveValue::Set(Some(zmessage.version as i32)),
            r#type: ActiveValue::Set(zmessage.r#type),
            public_key: ActiveValue::Set(Some(pub_key_hex)),
            data: ActiveValue::Set(zmessage.data),
            signature: ActiveValue::Set(Some(zmessage.signature)),
            from: ActiveValue::Set(from_hex),
            to: ActiveValue::Set(to_hex),
            ..Default::default()
        };
        let res = ZMessages::insert(zmessage).exec(self.pg_db.as_ref()).await;
        if let Err(err) = res {
            error!("Insert z_messages error, err: {}", err);
        }
    }

    pub async fn get_clock_by_msgid(&self, msg_id: &str) -> Result<ClockInfo, DbErr> {
        let clock_info = ClockInfos::find().filter(clock_infos::Column::MessageId.eq(msg_id)).one(self.pg_db.as_ref()).await;
        match clock_info {
            Err(err) => {
                error!("Query clockinfos by msg_id error, err: {}", err);
                Err(err)
            }
            Ok(None) => {
                let err = DbErr::RecordNotFound(format!("when msg_id is {}", msg_id));
                error!("RecordNotFound: Clock not found for msg_id: {}", msg_id);
                Err(err)
            }
            Ok(Some(clock)) => {
                let clock_ret: ClockInfo = clock.into();
                return Ok(clock_ret);
            }
        }
    }

    pub async fn get_last_clock(&self) -> Result<ClockInfo, DbErr> {
        let clock_info = ClockInfos::find()
            .order_by_desc(clock_infos::Column::Id)
            .one(self.pg_db.as_ref()).await;

        match clock_info {
            Err(err) => {
                error!("Query latest clockinfos error, err: {}", err);
                Err(err)
            }
            Ok(None) => {
                let err = DbErr::RecordNotFound(format!("empty clockinfo table"));
                info!("RecordNotFound: empty clockinfo table");
                Err(err)
            }
            Ok(Some(clock)) => {
                let clock_ret: ClockInfo = clock.into();
                return Ok(clock_ret);
            }
        }
    }

    pub async fn get_p2pmsg_by_msgid(&self, msg_id: &str) -> Result<ProtoZMessage, DbErr> {
        let p2p_msg = ZMessages::find().filter(z_messages::Column::MessageId.eq(msg_id)).one(self.pg_db.as_ref()).await;
        match p2p_msg {
            Err(err) => {
                error!("Query zmessages by msg_id error, err: {}", err);
                Err(err)
            }
            Ok(None) => {
                let err = DbErr::RecordNotFound(format!("when msg_id is {}", msg_id));
                error!("RecordNotFound: ZMessage not found for msg_id: {}", msg_id);
                Err(err)
            }
            Ok(Some(zmessage)) => {
                let msg = self.model_to_zmessage(zmessage);
                return Ok(msg);
            }
        }
    }

    pub async fn get_clocks_by_keyid(&self, start_id: u64, number: u64) -> Result<Vec<ClockInfo>, DbErr> {
        let clock_infos= ClockInfos::find()
            .filter(clock_infos::Column::Id.gt(start_id))
            .limit(number)
            .all(self.pg_db.as_ref()).await;

        match clock_infos {
            Err(err) => {
                error!("Query clockinfos by start_id error, err: {}", err);
                Err(err)
            }
            Ok(clocks) => {
                let clock_rets = clocks.iter().map(|clock| (clock.clone().into())).collect();
                return Ok(clock_rets);
            }
        }
    }

    pub async fn get_mergelogs_by_keyid(&self, start_id: u64, number: u64) -> Result<Vec<MergeLog>, DbErr> {
        let merge_logs= MergeLogs::find()
            .filter(merge_logs::Column::Id.gt(start_id))
            .limit(number)
            .all(self.pg_db.as_ref()).await;

        match merge_logs {
            Err(err) => {
                error!("Query merge_logs by start_id error, err: {}", err);
                Err(err)
            }
            Ok(logs) => {
                let mergelog_rets = logs.iter().map(|log| (log.clone().into())).collect();
                return Ok(mergelog_rets);
            }
        }
    }

    pub async fn get_zmessages_by_keyid(&self, start_id: u64, number: u64) -> Result<Vec<ProtoZMessage>, DbErr> {
        let zmessage= ZMessages::find()
            .filter(z_messages::Column::Id.gt(start_id))
            .limit(number)
            .all(self.pg_db.as_ref()).await;

        match zmessage {
            Err(err) => {
                error!("Query z_messages by start_id error, err: {}", err);
                Err(err)
            }
            Ok(zmessages) => {
                let zmessage_rets = zmessages.iter().map(|msg| self.model_to_zmessage(msg.clone())).collect();
                return Ok(zmessage_rets);
            }
        }
    }

    pub async fn get_clocks_counts(&self) -> Result<u64, DbErr> {
        let clocks_count= ClockInfos::find()
            .count(self.pg_db.as_ref())
            .await;

        match clocks_count {
            Err(err) => {
                error!("Query clock_info counts error, err: {}", err);
                Err(err)
            }
            Ok(counts) => {
                return Ok(counts);
            }
        }
    }

    pub async fn get_mergelogs_counts(&self) -> Result<u64, DbErr> {
        let mergelog_count= MergeLogs::find()
            .count(self.pg_db.as_ref())
            .await;

        match mergelog_count {
            Err(err) => {
                error!("Query z_messages counts error, err: {}", err);
                Err(err)
            }
            Ok(counts) => {
                return Ok(counts);
            }
        }
    }

    pub async fn get_zmessages_counts(&self) -> Result<u64, DbErr> {
        let zmessage_count= ZMessages::find()
            .count(self.pg_db.as_ref())
            .await;

        match zmessage_count {
            Err(err) => {
                error!("Query z_messages counts error, err: {}", err);
                Err(err)
            }
            Ok(counts) => {
                return Ok(counts);
            }
        }
    }

    fn model_to_zmessage(&self, zmessage: z_messages::Model) -> ProtoZMessage {
        let msg_id = hex::decode(zmessage.message_id).unwrap_or_else(|_| Vec::new());
        let pub_key_bytes = hex::decode(zmessage.public_key.unwrap()).unwrap_or_else(|_| Vec::new());
        let from_bytes = hex::decode(zmessage.from).unwrap_or_else(|_| Vec::new());
        let to_bytes = hex::decode(zmessage.to).unwrap_or_else(|_| Vec::new());
        let msg = ProtoZMessage {
            id: msg_id,
            version: zmessage.version.unwrap() as u32,
            r#type: zmessage.r#type,
            public_key: pub_key_bytes,
            data: zmessage.data,
            signature: zmessage.signature.unwrap(),
            from: from_bytes,
            to: to_bytes,
        };
        msg
    }

}