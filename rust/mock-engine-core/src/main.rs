#![forbid(unsafe_code)]

use std::ffi::{OsStr, OsString};
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use fcitx5_mock_engine_core::{
    default_pipe, parse_options, response_for, ClientState, Options, ResponseContext,
};
use fcitx5_protocol_core as protocol;
use fcitx5_windows_common_core::{
    current_runtime_generation_for_current_process, deadline_after, CurrentUserRuntimeIdentity,
    NamedEvent, NamedPipeServer,
};

fn main() -> ExitCode {
    let mut arguments = std::env::args_os();
    let _program = arguments.next();
    if matches!(arguments.next().as_deref(), Some(value) if value == OsStr::new("--version"))
        && arguments.next().is_none()
    {
        println!(
            "fcitx5-mock-engine {} protocol {}",
            env!("CARGO_PKG_VERSION"),
            protocol::VERSION
        );
        return ExitCode::SUCCESS;
    }

    let arguments = std::env::args_os().skip(1);
    let options = match parse_options(arguments) {
        Ok(options) => options,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(1);
        }
    };
    match run(options) {
        Ok(()) => ExitCode::SUCCESS,
        Err(code) => ExitCode::from(code),
    }
}

fn run(options: Options) -> Result<(), u8> {
    if let Some(generation) = options.generation.as_deref() {
        let generation = generation.to_str().ok_or(1)?;
        std::env::set_var("FCITX5_RELEASE_GENERATION", generation);
        if current_runtime_generation_for_current_process() != generation {
            return Err(1);
        }
    }

    let identity = CurrentUserRuntimeIdentity::current().ok_or(4)?;
    let pipe_name = options.pipe.or_else(|| default_pipe(&identity)).ok_or(4)?;
    let security = identity.security_attributes().ok_or(4)?;
    let stop_name = options.stop_event.unwrap_or_else(|| {
        OsString::from(format!(
            "Local\\Fcitx5WindowsNext.MockEngine.Stop.{}.{}",
            identity.session_id(),
            std::process::id()
        ))
    });
    let stop = Arc::new(NamedEvent::create(&stop_name, &security).map_err(|_| 3)?);
    let ready = options
        .ready_event
        .as_deref()
        .map(|name| {
            let security = identity.security_attributes().ok_or(3)?;
            NamedEvent::create(name, &security).map_err(|_| 3)
        })
        .transpose()?;
    let ready = ready.map(Arc::new);
    let worker_count = if options.test_clients == 0 {
        4
    } else {
        options.test_clients
    };
    let epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| 3)?
        .as_nanos() as u64;
    let next_response_id = Arc::new(AtomicU64::new(1));
    let completed_clients = Arc::new(AtomicU32::new(0));
    let ready_signaled = Arc::new(AtomicBool::new(false));
    let mut workers = Vec::with_capacity(worker_count as usize);

    for _ in 0..worker_count {
        let pipe_name = pipe_name.clone();
        let identity = identity.clone();
        let stop = Arc::clone(&stop);
        let next_response_id = Arc::clone(&next_response_id);
        let completed_clients = Arc::clone(&completed_clients);
        let ready_signaled = Arc::clone(&ready_signaled);
        let ready = ready.as_ref().map(Arc::clone);
        let test_clients = options.test_clients;
        let composition_test = options.composition_test;
        workers.push(std::thread::spawn(move || {
            worker(
                &pipe_name,
                &identity,
                &stop,
                ready,
                &ready_signaled,
                &next_response_id,
                &completed_clients,
                test_clients,
                epoch,
                composition_test,
            )
        }));
    }
    for worker in workers {
        worker.join().map_err(|_| 2)??;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn worker(
    pipe_name: &OsStr,
    identity: &CurrentUserRuntimeIdentity,
    stop: &NamedEvent,
    ready: Option<Arc<NamedEvent>>,
    ready_signaled: &AtomicBool,
    next_response_id: &AtomicU64,
    completed_clients: &AtomicU32,
    test_clients: u32,
    epoch: u64,
    composition_test: bool,
) -> Result<(), u8> {
    let security = identity.security_attributes().ok_or(4)?;
    loop {
        if stop.is_signaled() {
            return Ok(());
        }
        let pipe = NamedPipeServer::create(pipe_name, &security, protocol::MAX_FRAME_SIZE)
            .map_err(|_| 2)?;
        if !ready_signaled.swap(true, Ordering::AcqRel) {
            if let Some(ready) = &ready {
                ready.signal().map_err(|_| 3)?;
            }
        }
        if !pipe.connect_until(deadline_after(60_000), stop) {
            continue;
        }
        if let Some(client_process_id) = pipe.verified_client_process_id(identity) {
            serve_client(
                &pipe,
                identity.session_id(),
                client_process_id,
                epoch,
                next_response_id,
                stop,
                composition_test,
            );
        }
        if test_clients != 0 && completed_clients.fetch_add(1, Ordering::AcqRel) + 1 >= test_clients
        {
            stop.signal().map_err(|_| 3)?;
            return Ok(());
        }
    }
}

fn serve_client(
    pipe: &NamedPipeServer,
    session_id: u32,
    client_process_id: u32,
    epoch: u64,
    next_response_id: &AtomicU64,
    stop: &NamedEvent,
    composition_test: bool,
) {
    let mut state = ClientState::default();
    loop {
        let Some(request) = read_frame(pipe, stop) else {
            return;
        };
        let response_id = next_response_id.fetch_add(1, Ordering::Relaxed);
        let Some(response) = response_for(
            &request,
            ResponseContext {
                epoch,
                response_id,
                session_id,
                client_process_id,
                composition_test,
            },
            &mut state,
        ) else {
            return;
        };
        if !pipe.write_all(&response, deadline_after(100)) {
            return;
        }
    }
}

fn read_frame(pipe: &NamedPipeServer, stop: &NamedEvent) -> Option<Vec<u8>> {
    let mut header = [0_u8; protocol::HEADER_SIZE];
    if !pipe.read_exact(&mut header, deadline_after(60_000)) {
        return None;
    }
    let (_, body_size, _) = protocol::decode_header(&header)?;
    let body_size = usize::try_from(body_size).ok()?;
    if body_size > protocol::MAX_FRAME_SIZE - protocol::HEADER_SIZE {
        return None;
    }
    let mut frame = Vec::with_capacity(protocol::HEADER_SIZE + body_size);
    frame.extend_from_slice(&header);
    frame.resize(protocol::HEADER_SIZE + body_size, 0);
    if body_size != 0 && !pipe.read_exact(&mut frame[protocol::HEADER_SIZE..], deadline_after(100))
    {
        return None;
    }
    if stop.is_signaled() {
        None
    } else {
        Some(frame)
    }
}
