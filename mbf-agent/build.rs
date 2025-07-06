use std::{fs::OpenOptions, path::Path};

const LIBMAIN_URL: &str =
    "https://github.com/sc2ad/LibMainLoader/releases/download/v0.1.0-alpha/libmain.so";
const SL2_URL: &str = "https://github.com/sc2ad/Scotland2/releases/latest/download/libsl2.so";
const OVR_URL: &str = "https://github.com/kodenamekrak/JusticeForQuest/raw/refs/heads/master/third_party/libovrplatformloader.so";

fn download_if_not_exist(url: &str, to: &str) {
    if Path::new(to).exists() {
        return;
    }

    println!("Downloading {url} to {to:?}");
    let mut reader = ureq::get(url)
        .call()
        .expect("Failed to access URL")
        .into_reader();
    let mut writer = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(to)
        .expect("Failed to create output file");

    std::io::copy(&mut reader, &mut writer).expect("Failed to download file");
}

fn main() {
    std::fs::create_dir_all("./libs").expect("Failed to create ./libs");
    download_if_not_exist(LIBMAIN_URL, "./libs/libmain.so");
    download_if_not_exist(SL2_URL, "./libs/libsl2.so");
    download_if_not_exist(OVR_URL, "./libs/libovrplatformloader.so");
}
