use mdns_sd::{ServiceDaemon, ServiceInfo};
use std::net::{UdpSocket, IpAddr};

fn get_local_ip() -> IpAddr {
    let socket = UdpSocket::bind("0.0.0.0:0").unwrap();
    socket.connect("8.8.8.8:80").unwrap();
    socket.local_addr().unwrap().ip()
}

pub fn start_mdns_service(port: u16) -> ServiceDaemon {
    let mdns = ServiceDaemon::new().expect("Failed to start mDNS");

    let service_type = "_pcloud._tcp.local.";
    let instance_name = "PCloud Server";
    let hostname = "pcloud.local.";

    let ip = get_local_ip();
    println!("{}", ip);

    let properties: &[(&str, &str)] = &[
        ("version", "1"),
        ("api", "/api/client"),
    ];

    let service = ServiceInfo::new(
        service_type,
        instance_name,
        hostname,
        ip,
        port,
        properties,
    )
    .unwrap();

    mdns.register(service).unwrap();

    println!("mDNS service advertised: {}", service_type);

    mdns
}