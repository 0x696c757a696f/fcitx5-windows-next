use std::time::Instant;

use fcitx5_protocol_core::{
    decode_frame, decode_key_request, encode_key_request, KeyRequest, Metadata,
};

fn main() {
    const ITERATIONS: u64 = 200_000;
    let started = Instant::now();
    let mut checksum = 0_u64;
    for index in 0..ITERATIONS {
        let input = KeyRequest {
            metadata: Metadata {
                request_id: index + 1,
                response_to: 0,
                engine_epoch: 1,
                session_id: 1,
                context_id: 7,
                composition_id: 0,
                revision: 0,
            },
            virtual_key: b'A' as u32,
            ..Default::default()
        };
        let bytes = encode_key_request(&input).expect("benchmark request should encode");
        let frame = decode_frame(&bytes).expect("benchmark frame should decode");
        let decoded = decode_key_request(&frame).expect("benchmark request should decode");
        checksum = checksum.wrapping_add(decoded.metadata.request_id);
    }
    let elapsed_seconds = started.elapsed().as_secs_f64();
    let ns_per_operation = elapsed_seconds * 1_000_000_000.0 / ITERATIONS as f64;
    let operations_per_second = ITERATIONS as f64 / elapsed_seconds;
    println!(
        "{{\"benchmark\":\"ipc_codec\",\"architecture_bits\":{},\"iterations\":{},\"ns_per_operation\":{},\"operations_per_second\":{},\"checksum\":{}}}",
        std::mem::size_of::<usize>() * 8,
        ITERATIONS,
        ns_per_operation,
        operations_per_second,
        checksum
    );
}
