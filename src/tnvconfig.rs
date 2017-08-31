extern crate config;

use config::Config;

pub fn rd_config (args: Vec<String>) -> Config {
    let mut config = Config::new();
    config
        .merge(config::File::with_name("config")).unwrap()
        .merge(config::Environment::with_prefix("APP")).unwrap();
//    let addr = SocketAddr::new(
//        IpAddr::from_str(&config.get_str("bind_addr").unwrap()).unwrap(),
//        config.get_int("bind_port").unwrap() as u16);
    config
}
