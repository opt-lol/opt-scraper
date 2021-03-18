use clap::{App as ClapApp, Arg};
use std::fs::create_dir_all;
use std::path::Path;

mod app;
mod batch;

pub(crate) use app::App;
pub(crate) use batch::Batch;

fn main() {
    env_logger::init();
    let matches = ClapApp::new("opt-scraper")
        .arg(
        Arg::with_name("service_center")
                .required(true)
                .takes_value(true)
                .short("sc")
                .long("service_center")
                .default_value("YSC")
        )
        .arg(
            Arg::with_name("from_id")
                .required(true)
                .takes_value(true)
                .short("f")
                .long("from")
                .help("Starting id (leave the last 3 digits)")
                .default_value("2190042")
        )
        .arg(
            Arg::with_name("to_id")
                .required(true)
                .takes_value(true)
                .short("t")
                .long("to")
                .default_value("auto")
                .help("Either use \"auto\" to automatically detect the end or enter the stop id (leave the last 3 digits)")
        )
        .arg(
            Arg::with_name("save_path")
                .required(true)
                .takes_value(true)
                .short("p")
                .long("path")
                .default_value("./data")
        )
        .get_matches();

    let service_center = matches.value_of("service_center").unwrap();
    let path = Path::new(matches.value_of("save_path").unwrap());
    create_dir_all(&path).expect("Failed to create directory for \"save_path\"");
    let path = path.canonicalize().unwrap();

    let from_id = matches
        .value_of("from_id")
        .unwrap()
        .parse::<u64>()
        .expect("Failed to parse \"from_id\"");

    let to_id = match matches.value_of("to_id").unwrap() {
        "auto" => None,
        n => Some(n.parse::<u64>().expect("Failed to parse \"to_id\"")),
    };

    if let Some(to_id) = to_id {
        assert!(from_id < to_id, "\"from_id\" must be less than \"to_id\"");
    }

    let app_inst = App::new(service_center.to_string(), from_id, to_id, path);
    dbg!(&app_inst);

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(app_inst.run());
}
