use std::net::SocketAddr;
use crate::vlc::ClockInfo;
use protos::innermsg::Innermsg;
use protos::vlc::{ClockType, EventTrigger, ZClock};
use protos::zmessage::{ZMessage, ZType};
use protos::bussiness::ZChat;
use prost::Message;
use crate::zchronod::ZchronodArc;
use tracing::*;

use super::response::{broadcast_srv_state, clockinfo_to_proto};

pub async fn handle_cli_write_msg(arc_zchronod: ZchronodArc,mut inner_msg: Innermsg, p2p_msg: &ZMessage, src: SocketAddr) {
    match p2p_msg.r#type() {
        ZType::Zchat =>{
            let zchat_msg = prost::bytes::Bytes::from(p2p_msg.data.clone());
            let m = ZChat::decode(zchat_msg).unwrap();
            let write_success = arc_zchronod.state.write().await.add(vec![p2p_msg.clone()]); 
            if write_success {
                let update_clock_info: ClockInfo = arc_zchronod.state.read().await.clock_info.clone();
                let state_storage = &arc_zchronod.clone().storage;
                state_storage.sinker_clock(hex::encode(p2p_msg.id.clone()),m.message_data, &update_clock_info).await;
                state_storage.sinker_zmessage(p2p_msg.clone()).await;
                let z_clock = make_event_trigger_zclock(arc_zchronod.clone(), p2p_msg).await;
                let mut z_msg = inner_msg.message.unwrap();
                z_msg.r#type = ZType::Clock.into();
                inner_msg.message = Some(z_msg);
                broadcast_srv_state(arc_zchronod, inner_msg, &z_clock.encode_to_vec(), src).await;
            }
        }
        _ => info!("Write: now just support ZType::Zchat = 4!"),
    }
}

pub async fn handle_srv_event_trigger(arc_zchronod: ZchronodArc, z_clock: ZClock, inner_msg: Innermsg, p2p_msg: &ZMessage,src: SocketAddr) {
    let event_msg = prost::bytes::Bytes::from(z_clock.data.clone());
    let event = EventTrigger::decode(event_msg).unwrap();
    let prost_clock = event.clock_info.unwrap();
    let input_clock_info :ClockInfo = (&prost_clock).into();
    let (need_broadcast, merged) = arc_zchronod.state.write().await.merge(input_clock_info.clone(), &vec!(event.message.clone().unwrap()));
    if need_broadcast {
        let new_z_clock = make_event_trigger_zclock(arc_zchronod.clone(), &event.message.unwrap()).await;
        broadcast_srv_state(arc_zchronod.clone(), inner_msg, &new_z_clock.encode_to_vec(), src).await;
    } else {
        info!("clock is bigger or equal, no actions");
    }
    if merged {
        let state_clock_info = &arc_zchronod.state.read().await.clock_info.clone();
        arc_zchronod.storage.sinker_clock(state_clock_info.message_id.clone(), vec![], &state_clock_info).await;
        arc_zchronod.storage.sinker_merge_log(&input_clock_info, state_clock_info).await;
        arc_zchronod.storage.sinker_zmessage(p2p_msg.clone()).await;
    }
}

async fn make_event_trigger_zclock(arc_zchronod: ZchronodArc, inner_p2p_msg: &ZMessage) -> ZClock {
    let proto_clock = Some(arc_zchronod.state.read().await.clock_info.clone()).map(clockinfo_to_proto());
    let event = EventTrigger {
        clock_info: proto_clock,
        message: Some(inner_p2p_msg.clone())
    };
    let z_clock = ZClock {
        r#type: ClockType::EventTrigger.into(),
        data: event.encode_to_vec(),
    };
    z_clock
}

// fn handle_event_trigger(&mut self, msg: EventTrigger) -> Option<ServerMessage> {
//     match self.clock_info.clock.partial_cmp(&msg.clock_info.clock) {
//         Some(cmp::Ordering::Equal) => None,
//         Some(cmp::Ordering::Greater) => None,
//         Some(cmp::Ordering::Less) => {
//             let self_clock_info = ClockInfo::new(self.clock_info.clock.clone(), self.id, self.clock_info.message_id.clone(), 0);
//             let req = ServerMessage::DiffReq(DiffReq { from_clock: self_clock_info, to:  msg.clock_info.id });
//             Some(req)
//         }
//         None => {
//             let base = self.clock_info.clock.base_common(&msg.clock_info.clock);
//             if base.is_genesis() {
//                 let req = ServerMessage::ActiveSync(ActiveSync { diffs: self.items.clone(), latest: self.clock_info.clone(), to: msg.clock_info.id });
//                 Some(req)
//             } else {
//                 if let Some(start_msg_id) = self.clock_to_msgid.get(&base.index_key()) {
//                     if let Some(diff_msg_ids) = get_suffix(&self.items, start_msg_id.to_string()) {
//                         // req diff and send self diff
//                         let req = ServerMessage::ActiveSync(ActiveSync { diffs: diff_msg_ids, latest: self.clock_info.clone(), to: msg.clock_info.id });
//                         Some(req)
//                     } else {
//                         None
//                     }
//                 } else {
//                     None
//                 }
//             }
//         }
//     }
// }
