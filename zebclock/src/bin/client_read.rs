use prost::Message;
use protos::{
    bussiness::{
        GatewayType, QueryByMsgId, QueryByTableKeyId, QueryMethod, QueryResponse, ZGateway,
    },
    innermsg::{Action, Identity, Innermsg},
    zmessage::{ZMessage, ZType},
    // vlc::ClockInfos
};
use std::{net::UdpSocket, sync::mpsc, thread, time::{Duration, Instant}};

fn main() -> std::io::Result<()> {
    let socket = UdpSocket::bind("127.0.0.1:0").expect("couldn't bind to address");
    let query_count = 5;

    // now support message: five query as follows
    // let msg_type = "by_msg_id_clock";
    // let msg_type = "by_msg_id_zmessage";
    let msg_type = "by_key_id_clockinfos";
    // let msg_type = "by_key_id_mergelogs";
    // let msg_type = "by_key_id_zmessages";
    // let msg_type = "query_status";

    let mut data = Vec::new();
    if msg_type == "by_msg_id_clock" {
        data = query_by_msg_id(GatewayType::ClockNode);
    } else if msg_type == "by_msg_id_zmessage" {
        data = query_by_msg_id(GatewayType::ZMessage);
    } else if msg_type == "by_key_id_clockinfos" {
        data = query_by_key_id(GatewayType::ClockNode);
    } else if msg_type == "by_key_id_mergelogs" {
        data = query_by_key_id(GatewayType::MergeLog);
    } else if msg_type == "by_key_id_zmessages" {
        data = query_by_key_id(GatewayType::ZMessage);
    } else if msg_type == "query_status" {
        data = query_status();
    }

    let destination = "127.0.0.1:8050";
    let copy_socket = socket.try_clone().unwrap();
    let (tx, rx) = mpsc::channel();
    let spawn = thread::spawn(move || {
        let start = Instant::now();
        for i in 0..query_count {
            copy_socket.send_to(&data, destination).expect("couldn't send data");
            if i % 100 == 0 {
                thread::sleep(Duration::from_millis(100));   // Control sending rate
            }
        }
        let duration = start.elapsed();

        // Notify the receiver thread that sending is done
        tx.send(duration).expect("Couldn't notify receiver thread");
    });

    // recv msg
    for index in 0..query_count {
        let mut buf = [0; 65535];
        match socket.recv_from(&mut buf) {
            Ok((size, _)) => {
                let msg = prost::bytes::Bytes::copy_from_slice(&buf[..size]);
                let response = Innermsg::decode(msg).unwrap();
                let ret = QueryResponse::decode(response.message.unwrap().data.as_ref()).unwrap();
                // let clocks = ClockInfos::decode(ret.data.as_ref()).unwrap();
                println!("Received response: {:?}", ret);
                // println!("clocks: {:?}", clocks);
            }
            Err(ref err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                println!("No response received.");
            }
            Err(err) => {
                eprintln!("Error receiving response: {}", err);
            }
        }
        println!("received msg is {} times", index+1);

        if let Ok(elapsed) = rx.try_recv() {
            println!("Send Done: {} query times in {:?}", query_count, elapsed);
        }
    }

    spawn.join().unwrap();
    Ok(())
}

fn query_by_msg_id(gw_type: GatewayType) -> Vec<u8> {
    let msg_id = "696e746f6279746573";
    let params = QueryByMsgId {
        msg_id: msg_id.to_owned(),
    };

    let mut buf1 = vec![];
    params.encode(&mut buf1).unwrap();

    let gateway = ZGateway {
        request_id: "query_by_msg_id_for_test".into(),
        r#type: gw_type.into(),
        method: QueryMethod::QueryByMsgid.into(),
        data: buf1,
    };

    let mut buf2 = vec![];
    gateway.encode(&mut buf2).unwrap();
    println!("buf2: {:?}", buf2);

    let p2p_msg = ZMessage {
        r#type: ZType::Gateway.into(),
        data: buf2,
        ..Default::default()
    };

    let inner_msg = Innermsg {
        identity: Identity::Client.into(),
        action: Action::Read.into(),
        message: Some(p2p_msg),
        ..Default::default()
    };

    let mut buf3 = vec![];
    inner_msg.encode(&mut buf3).unwrap();
    println!("buf3: {:?}", buf3);
    buf3
}

fn query_by_key_id(gw_type: GatewayType) -> Vec<u8> {
    let start_id = 0;
    let params = QueryByTableKeyId { last_pos: start_id };

    let gateway = ZGateway {
        request_id: "query_by_key_id_for_test".into(),
        r#type: gw_type.into(),
        method: QueryMethod::QueryByTableKeyid.into(),
        data: params.encode_to_vec(),
    };

    let p2p_msg = ZMessage {
        r#type: ZType::Gateway.into(),
        data: gateway.encode_to_vec(),
        ..Default::default()
    };

    let inner_msg = Innermsg {
        identity: Identity::Client.into(),
        action: Action::Read.into(),
        message: Some(p2p_msg),
        ..Default::default()
    };

    let mut buf3 = vec![];
    inner_msg.encode(&mut buf3).unwrap();
    println!("buf3: {:?}", buf3);
    buf3
}

fn query_status() -> Vec<u8> {
    let gateway = ZGateway {
        request_id: "query_status_for_test".into(),
        r#type: 0.into(),
        method: QueryMethod::QueryStatus.into(),
        data: vec![],
    };

    let p2p_msg = ZMessage {
        r#type: ZType::Gateway.into(),
        data: gateway.encode_to_vec(),
        ..Default::default()
    };

    let inner_msg = Innermsg {
        identity: Identity::Client.into(),
        action: Action::Read.into(),
        message: Some(p2p_msg),
        ..Default::default()
    };

    let mut buf3 = vec![];
    inner_msg.encode(&mut buf3).unwrap();
    println!("buf3: {:?}", buf3);
    buf3
}
