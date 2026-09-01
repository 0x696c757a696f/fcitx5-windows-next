#![forbid(unsafe_code)]

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitCode};
use std::time::{Duration, Instant};

use fcitx5_measurement_core::format_key_roundtrip_result;
use fcitx5_protocol_core as protocol;
use fcitx5_windows_common_core::{
    deadline_after, CurrentUserRuntimeIdentity, NamedEvent, VerifiedPipeClient,
};

const WARMUP_COUNT: usize = 100;
const SAMPLE_COUNT: usize = 2_000;
const IO_TIMEOUT: Duration = Duration::from_millis(2_000);

fn main() -> ExitCode {
    match run(std::env::args_os().skip(1)) {
        Ok(result) => {
            println!("{result}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(1)
        }
    }
}

fn run<I>(mut arguments: I) -> Result<String, String>
where
    I: Iterator<Item = OsString>,
{
    let engine_argument = arguments
        .next()
        .ok_or_else(|| "engine executable argument required".to_owned())?;
    if arguments.next().is_some() {
        return Err("exactly one engine executable argument is required".to_owned());
    }
    let engine_path = PathBuf::from(engine_argument);
    let expected_server = engine_path.clone();
    let identity = CurrentUserRuntimeIdentity::current()
        .ok_or_else(|| "failed to resolve current user runtime identity".to_owned())?;
    let security = identity
        .security_attributes()
        .ok_or_else(|| "failed to create benchmark object security".to_owned())?;
    let process_id = std::process::id();
    let pipe_name = OsString::from(format!(r"\\.\pipe\Fcitx5WindowsNext.Bench.{process_id}"));
    let ready_name = OsString::from(format!("Local\\Fcitx5WindowsNext.Bench.Ready.{process_id}"));
    let ready = NamedEvent::create(&ready_name, &security)
        .map_err(|error| format!("failed to create readiness event: {error}"))?;
    let mut child = launch_engine(&engine_path, &pipe_name, &ready_name)?;
    let result = measure(&pipe_name, &expected_server, &identity, &ready);
    let exit = stop_engine(&mut child);
    result.and_then(|json| {
        if exit {
            Ok(json)
        } else {
            Err("mock engine did not exit successfully".to_owned())
        }
    })
}

fn launch_engine(engine: &Path, pipe: &OsStr, ready: &OsStr) -> Result<Child, String> {
    use std::os::windows::process::CommandExt;

    Command::new(engine)
        .args([
            OsString::from("--test-once"),
            OsString::from("--pipe"),
            pipe.to_owned(),
            OsString::from("--ready-event"),
            ready.to_owned(),
        ])
        .creation_flags(0x0800_0000)
        .spawn()
        .map_err(|error| format!("failed to start mock engine: {error}"))
}

fn measure(
    pipe_name: &OsStr,
    expected_server: &Path,
    identity: &CurrentUserRuntimeIdentity,
    ready: &NamedEvent,
) -> Result<String, String> {
    let deadline = deadline_after(2_000);
    while !ready.is_signaled() && fcitx5_windows_common_core::deadline_has_time_remaining(deadline)
    {
        std::thread::sleep(Duration::from_millis(1));
    }
    if !ready.is_signaled() {
        return Err("mock engine readiness timed out".to_owned());
    }
    let mut client = VerifiedPipeClient::connect_exact(pipe_name, expected_server, IO_TIMEOUT)
        .ok_or_else(|| "failed to connect to verified mock engine pipe".to_owned())?;
    let mut response = vec![0_u8; protocol::MAX_FRAME_SIZE];
    let hello_id = 1;
    let hello = protocol::encode_hello_request(&protocol::HelloRequest {
        metadata: protocol::Metadata {
            request_id: hello_id,
            session_id: identity.session_id(),
            ..protocol::Metadata::default()
        },
        client_architecture_bits: (std::mem::size_of::<usize>() * 8) as u32,
        client_process_id: identity.process_id(),
    })
    .ok_or_else(|| "failed to encode hello request".to_owned())?;
    let response_len = client
        .transact(&hello, &mut response, IO_TIMEOUT)
        .ok_or_else(|| "hello roundtrip failed".to_owned())?;
    let hello_response = protocol::decode_hello_response(
        &protocol::decode_frame(&response[..response_len]).ok_or("invalid hello response")?,
    )
    .ok_or_else(|| "invalid typed hello response".to_owned())?;
    if hello_response.status != protocol::Status::Ok
        || hello_response.metadata.response_to != hello_id
    {
        return Err("hello response rejected".to_owned());
    }

    let epoch = hello_response.metadata.engine_epoch;
    let mut samples = Vec::with_capacity(SAMPLE_COUNT);
    for index in 0..WARMUP_COUNT + SAMPLE_COUNT {
        let request_id = (index + 2) as u64;
        let request = protocol::encode_key_request(&protocol::KeyRequest {
            metadata: protocol::Metadata {
                request_id,
                engine_epoch: epoch,
                session_id: identity.session_id(),
                context_id: 1,
                ..protocol::Metadata::default()
            },
            virtual_key: u32::from(b'A'),
            scan_code: 7,
            ..protocol::KeyRequest::default()
        })
        .ok_or_else(|| "failed to encode key request".to_owned())?;
        let started = Instant::now();
        let response_len = client
            .transact(&request, &mut response, IO_TIMEOUT)
            .ok_or_else(|| "key roundtrip failed".to_owned())?;
        let elapsed = started.elapsed().as_secs_f64() * 1_000_000.0;
        let key_response = protocol::decode_key_response(
            &protocol::decode_frame(&response[..response_len]).ok_or("invalid key response")?,
        )
        .ok_or_else(|| "invalid typed key response".to_owned())?;
        if key_response.status != protocol::Status::Ok
            || key_response.metadata.response_to != request_id
            || !key_response.handled
            || key_response.commit_utf8 != b"a"
        {
            return Err("key response did not match the roundtrip contract".to_owned());
        }
        if index >= WARMUP_COUNT {
            samples.push(elapsed);
        }
    }
    format_key_roundtrip_result(std::mem::size_of::<usize>() * 8, &samples)
        .ok_or_else(|| "benchmark produced no samples".to_owned())
}

fn stop_engine(child: &mut Child) -> bool {
    let deadline = Instant::now() + IO_TIMEOUT;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return status.success(),
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(1));
            }
            Ok(None) | Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                return false;
            }
        }
    }
}
