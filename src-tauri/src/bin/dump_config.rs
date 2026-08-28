use aether_desktop_lib::settings::SettingsStorage;
use aether_desktop_lib::routing::SingBoxConfigGenerator;

fn main() {
    let settings = SettingsStorage::load();
    let config = SingBoxConfigGenerator::generate(&settings);
    let json = SingBoxConfigGenerator::to_json_string(&config).unwrap();
    let path = SettingsStorage::get_singbox_config_path();
    std::fs::write(&path, json).unwrap();
    println!("sing-box config successfully generated to: {:?}", path);
}