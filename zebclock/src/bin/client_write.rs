use prost::Message;
use protos::{
    bussiness::ZChat,
    innermsg::{Action, Identity, Innermsg},
    vlc::{Clock, ClockInfo, ClockType, EventTrigger, ZClock},
    zmessage::{ZMessage, ZType},
};
use std::{
    collections::HashMap,
    net::UdpSocket, thread,
};

fn main() -> std::io::Result<()> {
    let socket = UdpSocket::bind("127.0.0.1:0").expect("couldn't bind to address");
    let write_count = 2;

    // now support message: srv_event_trigger_message
    let msg_type = "client";                    // first step test
    // let msg_type = "srv_event_trigger_message";    // second step test

    let mut buf3 = Vec::new();
    if msg_type == "client" {
        buf3 = client_message();
    } else if msg_type == "srv_event_trigger_message" {
        buf3 = srv_event_trigger_message();
    }

    let destination = "127.0.0.1:8050";
    let copy_socket = socket.try_clone().unwrap();
    let spawn = thread::spawn(move || {
        for _ in 0..write_count {
            copy_socket.send_to(&buf3, destination).expect("couldn't send data");
        }
    });

    // recv msg
    for index in 0..write_count {
        let mut buf = [0; 1024];
        match socket.recv_from(&mut buf) {
            Ok((size, _)) => {
                let msg = prost::bytes::Bytes::copy_from_slice(&buf[..size]);
                let response = Innermsg::decode(msg).unwrap();
                println!("Received response: {:?}", response);
                let zclock_bytes = prost::bytes::Bytes::from(response.message.unwrap().data);
                let zclock = ZClock::decode(zclock_bytes).unwrap();
                println!("Received zclock: {:?}", zclock);
                let ev_bytes = prost::bytes::Bytes::from(zclock.data);
                let event_trigger = EventTrigger::decode(ev_bytes).unwrap();
                println!("Received event_trigger: {:?}", event_trigger);
                let zchat_bytes = prost::bytes::Bytes::from(event_trigger.message.unwrap().data);
                let zchat = ZChat::decode(zchat_bytes).unwrap();
                println!("Received zchat: {:?}", zchat);
                println!("Received message: {:?}", String::from_utf8_lossy(&zchat.message_data).into_owned());
            }
            Err(ref err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                println!("No response received.");
            }
            Err(err) => {
                eprintln!("Error receiving response: {}", err);
            }
        }
        println!("received msg is {} times", index+1);
    }

    spawn.join().unwrap();
    Ok(())
}

fn client_message() -> Vec<u8> {
    let clock_info = make_clock_info();

    let zchat = ZChat {
        message_data: Vec::from("hello"),
        clock: Some(clock_info),
    };

    let mut buf2 = vec![];
    zchat.encode(&mut buf2).unwrap();
    println!("buf2: {:?}", buf2);

    let p2p_msg = ZMessage {
        id: hex::decode("78f16ee259fe7a23059890d81304476a75e6ba9df74b624226553c7fd545bcb2").unwrap(),
        from: Vec::from("msgfrom"),
        to: Vec::from("msg.to"),
        r#type: ZType::Zchat.into(),
        data: buf2,
        ..Default::default()
    };

    let inner_msg = Innermsg {
        identity: Identity::Client.into(),
        action: Action::Write.into(),
        message: Some(p2p_msg),
        ..Default::default()
    };

    let mut buf3 = vec![];
    inner_msg.encode(&mut buf3).unwrap();
    println!("buf: {:?}", buf3);
    buf3
}


fn srv_event_trigger_message() -> Vec<u8> {
    let clock_info = make_clock_info();

    let zchat = ZChat {
        message_data: Vec::from("hello"),
        clock: None,
    };

    // empty data zmessage
    let inner_state_zmsg = ZMessage {
        id: Vec::from("intobytes"),
        from: Vec::from("msgfrom"),
        to: Vec::from("msg.to"),
        r#type: ZType::Zchat.into(),
        data: zchat.encode_to_vec(),
        ..Default::default()
    };

    let event = EventTrigger {
        clock_info: Some(clock_info),
        message: Some(inner_state_zmsg.clone()),
    };
    let z_clock = ZClock {
        r#type: ClockType::EventTrigger.into(),
        data: event.encode_to_vec(),
    };

    let p2p_msg = ZMessage {
        id: Vec::from("intobytes"),
        from: Vec::from("msgfrom"),
        to: Vec::from("msg.to"),
        r#type: ZType::Clock.into(),
        data: z_clock.encode_to_vec(),
        ..Default::default()
    };

    let inner_msg = Innermsg {
        identity: Identity::Server.into(),
        action: Action::Write.into(),
        message: Some(p2p_msg),
        ..Default::default()
    };

    let mut buf3 = vec![];
    inner_msg.encode(&mut buf3).unwrap();
    println!("buf: {:?}", buf3);
    buf3
}

fn make_clock_info() -> ClockInfo {
    let mut values = HashMap::new();
    values.insert("one".to_owned(), 1);

    let clock = Some(Clock { values });
    let id = Vec::from("one");
    let message_id = Vec::from("message_id");
    let count = 0;
    let create_at = tools::helper::get_time_ms();

    let clock_info = ClockInfo {
        clock,
        node_id: id,
        clock_hash: Vec::new(),
        message_id,
        count,
        create_at: create_at.try_into().unwrap(),
    };
    clock_info
}