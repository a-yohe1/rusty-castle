use std::hint::black_box;
use std::time::Instant;

use ssdp_core::parse_datagram;

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
    let msearch = b"M-SEARCH * HTTP/1.1\r\n\
HOST: 239.255.255.250:1900\r\n\
MAN: \"ssdp:discover\"\r\n\
MX: 3\r\n\
ST: ssdp:all\r\n\
\r\n";

    let notify = b"NOTIFY * HTTP/1.1\r\n\
HOST: 239.255.255.250:1900\r\n\
NT: upnp:rootdevice\r\n\
NTS: ssdp:alive\r\n\
USN: uuid:550e8400-e29b-41d4-a716-446655440000::upnp:rootdevice\r\n\
LOCATION: http://192.168.1.1:80/desc.xml\r\n\
CACHE-CONTROL: max-age=1800\r\n\
SERVER: Linux/5.4 UPnP/1.1 TestDevice/1.0\r\n\
\r\n";

    bench("parse_msearch", 500_000, || {
        black_box(parse_datagram(black_box(msearch)).unwrap());
    });
    bench("parse_notify_alive", 500_000, || {
        black_box(parse_datagram(black_box(notify)).unwrap());
    });
}
