#![feature(async_closure)]

use std::{
    collections::HashMap,
    env, fs,
    process::{Command, Stdio},
    time::Duration,
};

use clap::{App, Arg};
use dotenv::dotenv;
use fantoccini::{elements::Element, ClientBuilder, Locator};
use prometheus::{Counter, Encoder, Gauge, Opts, Registry, TextEncoder};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use tokio::time::sleep;

#[macro_use]
extern crate log;

const ENV_CHOMEDRIVER_PORT: &str = "CHROMEDRIVER_PORT";
const DEFAULT_CHROMEDRIVER_PORT: u16 = 9515;
const ENV_HUAWEI_ROUTER_HOST: &str = "HUAWEI_ROUTER_HOST";
const DEFAULT_HUAWEI_ROUTER_HOST: &str = "192.168.8.1";
const ENV_DEVICE_PASSWORD: &str = "HUAWEI_ROUTER_PASS";

enum OutputFormats {
    Json,
    Prometheus,
    Silent,
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    dotenv().ok();

    let matches = App::new("huawei-metrics")
        .arg(
            Arg::with_name("format")
                .short("f")
                .takes_value(true)
                .default_value("json")
                .help("Output format to print on stdout")
                .possible_values(&["json", "prometheus", "silent"]),
        )
        .arg(
            Arg::with_name("chromedriver")
                .short("c")
                .help("Starts and kills own chromedriver instance"),
        )
        .arg(
            Arg::with_name("prometheus-out")
                .long("po")
                .help("File to write prometheus metrics to in addition to the stdout output")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("json-out")
                .long("jo")
                .help("File to write json metrics to in addition to the stdout output")
                .takes_value(true),
        )
        .get_matches();

    let format = match matches.value_of("format").unwrap() {
        "json" => OutputFormats::Json,
        "prometheus" => OutputFormats::Prometheus,
        "silent" => OutputFormats::Silent,
        _ => unreachable!(),
    };

    let port: u16 = {
        let v = env::var(ENV_CHOMEDRIVER_PORT);
        match v {
            Ok(s) => str::parse(&s).expect("Chromedriver port specified but unable to parse"),
            Err(_) => DEFAULT_CHROMEDRIVER_PORT,
        }
    };

    let start_own_cd = matches.is_present("chromedriver");
    let chromedriver = if start_own_cd {
        info!("Starting own chromedriver on port {}", port);
        let cd = Some(
            Command::new("chromedriver")
                .arg(format!("--port={}", port))
                .stdout(Stdio::null())
                .spawn()
                .unwrap(),
        );
        trace!("Sleeping 2s to let chromedriver initialize itself");
        sleep(Duration::from_millis(2000)).await;
        cd
    } else {
        info!("Using chromedriver at port {}", port);
        None
    };

    let router_host =
        env::var(ENV_HUAWEI_ROUTER_HOST).unwrap_or_else(|_| DEFAULT_HUAWEI_ROUTER_HOST.to_owned());

    info!("Using huawei router at {}", router_host);

    let router_pass = env::var(ENV_DEVICE_PASSWORD).expect("HUAWEI_ROUTER_PASS not set!");

    info!("Found password in env: {}", ENV_DEVICE_PASSWORD);

    debug!("Connecting to webdriver");

    let mut capabilities = Map::new();
    let mut chrome_options = Map::new();

    chrome_options.insert("args".to_string(), json!(["--headless"]));

    if let Ok(bin) = std::env::var("CHROME_BINARY") {
        chrome_options.insert("binary".to_string(), json!(bin));
    }

    capabilities.insert(
        "goog:chromeOptions".to_string(),
        Value::Object(chrome_options),
    );

    let mut c = ClientBuilder::native()
        .capabilities(capabilities)
        .connect(&format!("http://localhost:{}", port))
        .await
        .expect("failed to connect to WebDriver");

    debug!("Navigating to router web interface");
    c.goto(&format!("http://{}/html/index.html", router_host))
        .await
        .unwrap();

    c.find(Locator::Id("login_password"))
        .await
        .unwrap()
        .send_keys(&router_pass)
        .await
        .unwrap();
    debug!("Entered password");

    c.find(Locator::Id("login_btn"))
        .await
        .unwrap()
        .click()
        .await
        .unwrap();
    debug!("Clicked login button");

    debug!("Starting to wait for #menu_top_advanceset");
    c.wait_for_find(Locator::Id("menu_top_advanceset"))
        .await
        .unwrap();

    debug!("Found advanced menu, navigating to device information page");
    c.goto(&format!(
        "http://{}/html/content.html#deviceinformation",
        router_host
    ))
    .await
    .unwrap();

    debug!("Waiting for device information page content");
    c.wait_for_find(Locator::Id("deviceinformation_page"))
        .await
        .unwrap();

    info!("Successfully navigated to device information page");
    debug!("Sleeping 2s to ensure all data has loaded");
    sleep(Duration::from_millis(2000)).await;

    let info =
        extract_information(&mut c.find(Locator::Id("deviceinformation_page")).await.unwrap())
            .await;

    let json_out = serde_json::to_string_pretty(&info).unwrap();
    let prometheus_out = {
        let r = Registry::new();
        for (label, value) in &info {
            if let Some(Parsed {
                value: numeric_value,
                unit,
            }) = &value.parsed
            {
                let opts = Opts::new(
                    format!(
                        "{}_{}",
                        label.to_ascii_lowercase(),
                        unit.to_ascii_lowercase()
                    ),
                    value.label.clone(),
                );
                let opts = opts.namespace("huawei_metrics");
                match unit.as_str() {
                    "Mbps" | "Kbps" | "Bps" | "dBm" | "dB" => {
                        let gauge = Gauge::with_opts(opts).unwrap();
                        gauge.set(*numeric_value);
                        r.register(Box::new(gauge)).unwrap();
                    }
                    "MB" | "GB" | "KB" | "B" => {
                        let counter = Counter::with_opts(opts).unwrap();
                        counter.inc_by(*numeric_value);
                        r.register(Box::new(counter)).unwrap();
                    }
                    _ => {
                        warn!(
                            "Skipping {:?} because of unknown unit to metric conversion",
                            info
                        );
                    }
                }
            }
        }

        let mut buffer = vec![];
        let encoder = TextEncoder::new();
        let metric_families = r.gather();
        encoder.encode(&metric_families, &mut buffer).unwrap();
        String::from_utf8(buffer).unwrap()
    };

    match format {
        OutputFormats::Json => {
            println!("{}", json_out);
        }
        OutputFormats::Prometheus => {
            println!("{}", prometheus_out);
        }
        _ => {}
    }

    for (option, content) in [("prometheus-out", prometheus_out), ("json-out", json_out)] {
        if let Some(filepath) = matches.value_of(option) {
            trace!("Outputting {} to {}", option, filepath);
            fs::write(filepath, content).unwrap();
        }
    }

    debug!("Closing window");
    c.close_window().await.unwrap();
    c.close().await.unwrap();

    if let Some(mut c) = chromedriver {
        info!("Killing own chromedriver");
        c.kill().ok();
    };
}

/*
<div class="clearboth" style="padding-top:20px;">
    <div class="control-label" style="margin-top: 8px;">
        <span lang-id="deviceinformation.softwareVersion">Software version</span>
    </div>
    <div class="controls controls-content" id="di-SoftwareVersion">
        <span>11.0.1.2(H200SP3C9831)</span>
    </div>
</div>
*/

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Information {
    label_id: String,
    label: String,
    value_id: String,
    value: String,
    parsed: Option<Parsed>,
    hidden: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Parsed {
    value: f64,
    unit: String,
}

async fn extract_information(info_page: &mut Element) -> HashMap<String, Information> {
    let mut info = HashMap::new();

    let mut table = info_page.find(Locator::Css(".main_content")).await.unwrap();

    let rows = table.find_all(Locator::Css(".clearboth")).await.unwrap();
    for mut row in rows.into_iter() {
        trace!("{:?}", row.html(false).await);
        let hidden = row
            .attr("style")
            .await
            .unwrap()
            .map(|v| v.contains("display: none;"))
            .unwrap_or(false);

        let mut label = row.find(Locator::Css(".control-label")).await.unwrap();
        let mut span = label.find(Locator::Css("span")).await.unwrap();
        let label_id = span.attr("lang-id").await.unwrap().unwrap();
        let label = span.html(true).await.unwrap();
        let mut value = row.find(Locator::Css(".controls-content")).await.unwrap();
        let value_id = value.attr("id").await.unwrap().unwrap();
        let value = match value.find_all(Locator::Css("span")).await {
            Ok(elements) => {
                let mut contents = Vec::with_capacity(elements.len());
                for mut el in elements {
                    contents.push(el.html(true).await.unwrap());
                }
                contents.join("")
            }
            Err(e) => {
                warn!("Unable to extract value: {:#}", e);
                "".to_owned()
            }
        };

        let parsed = {
            let mut parsed = None;
            for unit in ["dB", "dBm", "GB", "MB", "KB", "Gbps", "Mbps", "Kbps", "B"] {
                parsed = try_parse(&value, unit);
                if parsed.is_some() {
                    break;
                }
            }
            if let Some(Parsed { value, unit }) = parsed.clone() {
                if unit == "Kbps" {
                    parsed = Some(Parsed {
                        value: value / 1024f64,
                        unit: "Mbps".to_string(),
                    });
                }
            }
            parsed
        };

        let row_info = Information {
            label_id: label_id.clone(),
            label,
            value_id,
            value,
            hidden,
            parsed,
        };
        trace!("Adding row: {:?}", row_info);
        info.insert(label_id.split_once(".").unwrap().1.to_owned(), row_info);
    }

    info
}

fn try_parse(value: impl AsRef<str>, unit: impl AsRef<str>) -> Option<Parsed> {
    let value = value.as_ref();
    let unit = unit.as_ref();
    if value.ends_with(unit) {
        Some(Parsed {
            value: value[0..(value.len() - unit.len())].parse().ok()?,
            unit: unit.to_string(),
        })
    } else {
        None
    }
}
