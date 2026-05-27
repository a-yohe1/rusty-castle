use std::hint::black_box;
use std::time::Instant;

use dlna_core::ProtocolInfoRef;
use media_http::{MediaHeadersRef, Method, parse_range_header, plan_media_response};

fn bench<F>(name: &str, iterations: u64, mut f: F)
where
    F: FnMut(),
{
    let started = Instant::now();
    for _ in 0..iterations {
        f();
    }
    let elapsed = started.elapsed();
    let ns_per_iter = elapsed.as_nanos() as f64 / iterations as f64;
    println!("{name}: {ns_per_iter:.2} ns/iter ({iterations} iterations)");
}

fn main() {
    bench("parse_range_header", 1_000_000, || {
        black_box(parse_range_header(black_box("bytes=1024-65535")).unwrap());
    });

    let headers = MediaHeadersRef {
        len: 4_294_967_296,
        content_type: "video/mp4",
        protocol_info: ProtocolInfoRef::sony_mp4(),
    };

    bench("plan_partial_get", 1_000_000, || {
        black_box(plan_media_response(
            black_box(Method::Get),
            black_box(headers),
            black_box(Some("bytes=1024-65535")),
        ));
    });
}
