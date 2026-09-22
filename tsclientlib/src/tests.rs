use once_cell::sync::Lazy;
use ts_bookkeeping::messages::s2c::InMessage;
use tsproto_packets::packets::{Direction, Flags, OutPacket, PacketType};

static TRACING: Lazy<()> = Lazy::new(|| tracing_subscriber::fmt().with_test_writer().init());

pub(crate) fn create_logger() { Lazy::force(&TRACING); }

fn parse_msg(msg: &str) -> InMessage {
	let header = OutPacket::new_with_dir(Direction::S2C, Flags::empty(), PacketType::Command);

	InMessage::new(&header.header(), msg.as_bytes()).unwrap()
}

fn test_iconid(input: &str, expected: u32) {
	create_logger();

	let msg = parse_msg(&format!(
		r#"initserver virtualserver_name=TeamSpeak\s]I[\sServer virtualserver_welcomemessage=Welcome\sto\sTeamSpeak,\scheck\s[URL]www.teamspeak.com[\/URL]\sfor\slatest\sinformation virtualserver_platform=Linux virtualserver_version=3.11.0\s[Build:\s1578903157] virtualserver_maxclients=32 virtualserver_created=1571572631 virtualserver_codec_encryption_mode=2 virtualserver_hostmessage virtualserver_hostmessage_mode=0 virtualserver_default_server_group=8 virtualserver_default_channel_group=8 virtualserver_hostbanner_url virtualserver_hostbanner_gfx_url virtualserver_hostbanner_gfx_interval=0 virtualserver_priority_speaker_dimm_modificator=-18.0000 virtualserver_id=1 virtualserver_hostbutton_tooltip virtualserver_hostbutton_url virtualserver_hostbutton_gfx_url virtualserver_name_phonetic virtualserver_ip=0.0.0.0,\s:: virtualserver_ask_for_privilegekey=0 virtualserver_hostbanner_mode=0 virtualserver_channel_temp_delete_delay_default=0 virtualserver_nickname client_nickname=TeamSpeakUser client_version=3.?.?\s[Build:\s5680278000] client_platform=Windows client_input_muted=0 client_output_muted=0 client_outputonly_muted=0 client_input_hardware=1 client_output_hardware=1 client_default_channel client_default_channel_password client_server_password client_meta_data client_version_sign=DX5NIYLvfJEUjuIbCidnoeozxIDRRkpq3I9vVMBmE9L2qnekOoBzSenkzsg2lC9CMv8K5hkEzhr2TYUYSwUXCg== client_security_hash client_key_offset=354 client_away=0 client_away_message client_nickname_phonetic client_default_token client_badges client_myteamspeak_id client_integrations client_active_integrations_info client_myteamspeak_avatar client_signed_badges acn=TeamSpeakUser aclid=2 pv=7 client_talk_power=75 client_needed_serverquery_view_power=75 virtualserver_icon_id={}"#,
		input
	));
	if let InMessage::InitServer(list) = msg {
		let cmd = list.iter().next().unwrap();
		assert_eq!(cmd.icon, ts_bookkeeping::IconId(expected));
	} else {
		panic!("Failed to parse as initserver");
	}
}

#[test]
fn normal_iconid() { test_iconid("96136942", 96136942); }

#[test]
fn negative_iconid() { test_iconid("-96136942", 4198830354); }

#[test]
fn big_iconid() { test_iconid("18446744073225738240", 3811153920); }

const STREAM_ID: &str = "f2f7de30-8488-4000-8000-000000000001";

#[test]
fn stream_started() {
	let msg = parse_msg(&format!(
		r"notifystreamstarted clid=1 id={} name=Wayland\sSource type=3 access=1 mode=1 bitrate=33920 viewer_limit=0 audio=0",
		STREAM_ID
	));
	if let InMessage::StreamStarted(list) = msg {
		assert_eq!(list.iter().count(), 1);
		let part = list.iter().next().unwrap();
		assert_eq!(part.client_id, ts_bookkeeping::ClientId(1));
		assert_eq!(part.stream_id, STREAM_ID);
		assert_eq!(part.name.as_deref(), Some("Wayland Source"));
		assert_eq!(part.stream_type, Some(3));
		assert_eq!(part.access, Some(1));
		assert_eq!(part.mode, Some(1));
		assert_eq!(part.bitrate, Some(33920));
		assert_eq!(part.viewer_limit, Some(0));
		assert_eq!(part.audio, Some(false));
		assert_eq!(part.return_code, None);
	} else {
		panic!("Expected StreamStarted, got {:?}", msg);
	}
}

#[test]
fn stream_info_multiple_entries() {
	let msg = parse_msg(&format!(
		"notifystreaminfo return_code=info-1 clid=13 id={} name=Renamed type=2 accessibility=1 mode=1 viewer=1 bitrate=29104 viewer_limit=0 audio=0|clid=13 id=f2f7de30-8488-4000-8000-000000000002 name=Test type=3 accessibility=2 mode=1 viewer=0 bitrate=33920 viewer_limit=5 audio=1",
		STREAM_ID
	));
	if let InMessage::StreamInfo(list) = msg {
		let parts: Vec<_> = list.iter().collect();
		assert_eq!(parts.len(), 2);
		assert_eq!(parts[0].client_id, ts_bookkeeping::ClientId(13));
		assert_eq!(parts[0].stream_id, STREAM_ID);
		assert_eq!(parts[0].name.as_deref(), Some("Renamed"));
		assert_eq!(parts[0].stream_type, Some(2));
		assert_eq!(parts[0].accessibility, Some(1));
		assert_eq!(parts[0].mode, Some(1));
		assert_eq!(parts[0].viewer_count, Some(1));
		assert_eq!(parts[0].bitrate, Some(29104));
		assert_eq!(parts[0].viewer_limit, Some(0));
		assert_eq!(parts[0].audio, Some(false));
		assert_eq!(parts[0].return_code.as_deref(), Some("info-1"));
		assert_eq!(parts[1].client_id, ts_bookkeeping::ClientId(13));
		assert_eq!(parts[1].stream_id, "f2f7de30-8488-4000-8000-000000000002");
		assert_eq!(parts[1].name.as_deref(), Some("Test"));
		assert_eq!(parts[1].stream_type, Some(3));
		assert_eq!(parts[1].accessibility, Some(2));
		assert_eq!(parts[1].viewer_count, Some(0));
		assert_eq!(parts[1].bitrate, Some(33920));
		assert_eq!(parts[1].viewer_limit, Some(5));
		assert_eq!(parts[1].audio, Some(true));
	} else {
		panic!("Expected StreamInfo, got {:?}", msg);
	}
}

#[test]
fn stream_updated_partial_fields() {
	for patch in [
		"",
		" name=Renamed return_code=patch-1",
		" name= audio=0",
		" accessibility=2",
		" access=1",
		" type=99 mode=99 viewer=2 bitrate=750000 viewer_limit=5 audio=1",
	] {
		let msg = parse_msg(&format!("notifystreamupdated clid=13 id={}{}", STREAM_ID, patch));
		if let InMessage::StreamUpdated(list) = msg {
			let part = list.iter().next().unwrap();
			assert_eq!(part.client_id, ts_bookkeeping::ClientId(13));
			assert_eq!(part.stream_id, STREAM_ID);
			assert_eq!(
				part.name.as_deref(),
				if patch.contains("Renamed") {
					Some("Renamed")
				} else if patch.contains("name=") {
					Some("")
				} else {
					None
				}
			);
			assert_eq!(part.access, if patch.contains("access=") { Some(1) } else { None });
			assert_eq!(
				part.accessibility,
				if patch.contains("accessibility=") { Some(2) } else { None }
			);
			assert_eq!(
				part.audio,
				if patch.contains("audio=") { Some(patch.contains("audio=1")) } else { None }
			);
			assert_eq!(part.stream_type, if patch.contains("type=") { Some(99) } else { None });
			assert_eq!(part.mode, if patch.contains("mode=") { Some(99) } else { None });
			assert_eq!(part.viewer_count, if patch.contains("viewer=") { Some(2) } else { None });
			assert_eq!(part.bitrate, if patch.contains("bitrate=") { Some(750000) } else { None });
			assert_eq!(
				part.viewer_limit,
				if patch.contains("viewer_limit=") { Some(5) } else { None }
			);
			assert_eq!(
				part.return_code.as_deref(),
				if patch.contains("return_code=") { Some("patch-1") } else { None }
			);
		} else {
			panic!("Expected StreamUpdated, got {:?}", msg);
		}
	}
}

#[test]
fn stream_membership_and_stop() {
	let msg = parse_msg(&format!("notifystreamclientjoined clid=10 id={}", STREAM_ID));
	if let InMessage::StreamClientJoined(list) = msg {
		let part = list.iter().next().unwrap();
		assert_eq!(part.client_id, ts_bookkeeping::ClientId(10));
		assert_eq!(part.stream_id, STREAM_ID);
	} else {
		panic!("Expected StreamClientJoined, got {:?}", msg);
	}
	for reason in ["", " reason=1", " reason=4", " reason=99"] {
		for command in ["notifystreamstopped", "notifystreamclientleft"] {
			let msg = parse_msg(&format!("{} clid=1 id={}{}", command, STREAM_ID, reason));
			let (client_id, stream_id, parsed_reason) = match &msg {
				InMessage::StreamStopped(list) => {
					assert_eq!(command, "notifystreamstopped");
					let part = list.iter().next().unwrap();
					(part.client_id, &part.stream_id, part.reason)
				}
				InMessage::StreamClientLeft(list) => {
					assert_eq!(command, "notifystreamclientleft");
					let part = list.iter().next().unwrap();
					(part.client_id, &part.stream_id, part.reason)
				}
				_ => panic!("Unexpected message {:?}", msg),
			};
			assert_eq!(client_id, ts_bookkeeping::ClientId(1));
			assert_eq!(stream_id, STREAM_ID);
			assert_eq!(
				parsed_reason,
				reason.strip_prefix(" reason=").map(|v| v.parse::<u32>().unwrap())
			);
		}
	}
}

#[test]
fn stream_join_response_offer_and_rejection() {
	for message in ["msg", "msg="] {
		let msg = parse_msg(&format!(
			r"notifyrespondjoinstreamrequest clid=1 id={} {} decision=1 offer=v=0\r\no=-\s1\s2\sIN\sIP4\s127.0.0.1\r\na=x:\p\/\\s\r\n",
			STREAM_ID, message
		));
		if let InMessage::RespondJoinStreamRequest(list) = msg {
			let part = list.iter().next().unwrap();
			assert_eq!(part.client_id, ts_bookkeeping::ClientId(1));
			assert_eq!(part.stream_id, STREAM_ID);
			assert_eq!(part.message.as_deref(), Some(""));
			assert_eq!(part.decision, 1);
			assert_eq!(
				part.offer.as_deref(),
				Some("v=0\r\no=- 1 2 IN IP4 127.0.0.1\r\na=x:|/\\s\r\n")
			);
		} else {
			panic!("Expected RespondJoinStreamRequest, got {:?}", msg);
		}
	}
	for message in ["", " msg", " msg=Denied"] {
		let msg = parse_msg(&format!(
			"notifyrespondjoinstreamrequest clid=1 id={} decision=0{}",
			STREAM_ID, message
		));
		if let InMessage::RespondJoinStreamRequest(list) = msg {
			let part = list.iter().next().unwrap();
			assert_eq!(part.decision, 0);
			assert_eq!(part.offer, None);
			assert_eq!(
				part.message.as_deref(),
				match message {
					"" => None,
					" msg" => Some(""),
					_ => Some("Denied"),
				}
			);
		} else {
			panic!("Expected RespondJoinStreamRequest, got {:?}", msg);
		}
	}
}

#[test]
fn stream_signaling_opaque_json() {
	// TS3 decoding must leave JSON escapes intact for the application to parse once.
	for (wire, decoded) in [
		(
			r#"{"args":{"mLine":0,"mid":"0","sdp":"candidate:1\styp\shost"},"cmd":"iceCandidate"}"#,
			r#"{"args":{"mLine":0,"mid":"0","sdp":"candidate:1 typ host"},"cmd":"iceCandidate"}"#,
		),
		(
			r#"{"cmd":"answer","args":{"answer":"v=0\\r\\na=x:\p\/\\\\s"}}"#,
			r#"{"cmd":"answer","args":{"answer":"v=0\r\na=x:|/\\s"}}"#,
		),
		(r#"{"cmd":"bogusCommand","args":{}}"#, r#"{"cmd":"bogusCommand","args":{}}"#),
		("not-json", "not-json"),
	] {
		let msg =
			parse_msg(&format!("notifystreamsignaling clid=1 id={} json={}", STREAM_ID, wire));
		if let InMessage::StreamSignaling(list) = msg {
			let part = list.iter().next().unwrap();
			assert_eq!(part.client_id, ts_bookkeeping::ClientId(1));
			assert_eq!(part.stream_id, STREAM_ID);
			assert_eq!(part.json, decoded);
		} else {
			panic!("Expected StreamSignaling, got {:?}", msg);
		}
	}
}

#[test]
fn stream_viewer_outbound() {
	use ts_bookkeeping::ClientId;
	use ts_bookkeeping::messages::{OutMessageTrait, c2s};

	for is_remove in [false, true] {
		let packet = c2s::OutJoinStreamRequestPart {
			stream_id: STREAM_ID.into(),
			client_id: ClientId(1),
			is_remove,
			message: "".into(),
		}
		.to_packet();
		// The existing serializer emits empty values as bare keys, not omitted keys.
		assert_eq!(
			packet.0.content(),
			format!(
				"joinstreamrequest id={} clid=1 is_remove={} msg",
				STREAM_ID,
				u8::from(is_remove)
			)
			.as_bytes()
		);
		let parsed = c2s::InMessage::new(&packet.0.header(), packet.0.content()).unwrap();
		if let c2s::InMessage::JoinStreamRequest(list) = parsed {
			let part = list.iter().next().unwrap();
			assert_eq!(part.stream_id, STREAM_ID);
			assert_eq!(part.client_id, ClientId(1));
			assert_eq!(part.is_remove, is_remove);
			assert_eq!(part.message, "");
		} else {
			panic!("Expected JoinStreamRequest, got {:?}", parsed);
		}
	}
	for stream_id in [None, Some(STREAM_ID.into())] {
		let packet =
			c2s::OutRequestStreamInfoPart { client_id: ClientId(13), stream_id: stream_id.clone() }
				.to_packet();
		let expected = if stream_id.is_some() {
			format!("requeststreaminfo clid=13 id={}", STREAM_ID)
		} else {
			"requeststreaminfo clid=13".into()
		};
		assert_eq!(packet.0.content(), expected.as_bytes());
	}
	let packet = c2s::OutStreamSignalingPart {
		client_id: ClientId(1),
		stream_id: STREAM_ID.into(),
		json: r#"{"cmd":"answer","args":{"answer":"v=0\r\na=x:|/\\s space"}}"#.into(),
	}
	.to_packet();
	assert_eq!(packet.0.content(), format!(r#"streamsignaling clid=1 id={} json={{"cmd":"answer","args":{{"answer":"v=0\\r\\na=x:\p\/\\\\s\sspace"}}}}"#, STREAM_ID).as_bytes());
}

#[test]
fn stream_join_request_from_viewer() {
	// Wire shape from the broadcaster capture (§9b.2): msg present but valueless.
	for (is_remove, flag) in [(false, "0"), (true, "1")] {
		let msg = parse_msg(&format!(
			"notifyjoinstreamrequest clid=2 id={} msg is_remove={}",
			STREAM_ID, flag
		));
		if let InMessage::JoinStreamRequest(list) = msg {
			let part = list.iter().next().unwrap();
			assert_eq!(part.client_id, ts_bookkeeping::ClientId(2));
			assert_eq!(part.stream_id, STREAM_ID);
			assert_eq!(part.message.as_deref(), Some(""));
			assert_eq!(part.is_remove, is_remove);
		} else {
			panic!("Expected JoinStreamRequest, got {:?}", msg);
		}
	}
}

#[test]
fn stream_broadcaster_outbound() {
	use ts_bookkeeping::ClientId;
	use ts_bookkeeping::messages::{OutMessageTrait, c2s};

	let packet = c2s::OutSetupStreamPart {
		name: Some("Bot Stream".into()),
		stream_type: Some(2),
		mode: Some(1),
		bitrate: Some(750000),
		viewer_limit: Some(0),
		audio: Some(false),
		accessibility: 1,
	}
	.to_packet();
	assert_eq!(
		packet.0.content(),
		br"setupstream name=Bot\sStream type=2 mode=1 bitrate=750000 viewer_limit=0 audio=0 accessibility=1"
	);
	// Everything but accessibility is optional.
	let packet = c2s::OutSetupStreamPart {
		name: None,
		stream_type: None,
		mode: None,
		bitrate: None,
		viewer_limit: None,
		audio: None,
		accessibility: 1,
	}
	.to_packet();
	assert_eq!(packet.0.content(), b"setupstream accessibility=1");

	let packet = c2s::OutUpdateStreamPart {
		stream_id: STREAM_ID.into(),
		name: Some("Renamed".into()),
		stream_type: None,
		mode: None,
		bitrate: None,
		viewer_limit: None,
		audio: None,
		accessibility: None,
	}
	.to_packet();
	assert_eq!(packet.0.content(), format!("updatestream id={} name=Renamed", STREAM_ID).as_bytes());

	let packet = c2s::OutStopStreamPart { stream_id: STREAM_ID.into(), reason: 1 }.to_packet();
	assert_eq!(packet.0.content(), format!("stopstream id={} reason=1", STREAM_ID).as_bytes());

	let packet = c2s::OutRespondJoinStreamRequestPart {
		client_id: ClientId(2),
		stream_id: STREAM_ID.into(),
		decision: 1,
		message: "".into(),
		offer: "v=0\r\no=- 1 2 IN IP4 127.0.0.1\r\na=x:|/\\s\r\n".into(),
	}
	.to_packet();
	assert_eq!(
		packet.0.content(),
		format!(
			r"respondjoinstreamrequest clid=2 id={} decision=1 msg offer=v=0\r\no=-\s1\s2\sIN\sIP4\s127.0.0.1\r\na=x:\p\/\\s\r\n",
			STREAM_ID
		)
		.as_bytes()
	);

	let packet = c2s::OutRemoveClientFromStreamPart {
		client_id: ClientId(2),
		stream_id: STREAM_ID.into(),
		reason: 4,
	}
	.to_packet();
	assert_eq!(
		packet.0.content(),
		format!("removeclientfromstream clid=2 id={} reason=4", STREAM_ID).as_bytes()
	);
}
