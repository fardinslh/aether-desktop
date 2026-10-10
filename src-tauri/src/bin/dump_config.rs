use aether_desktop_lib::routing::SingBoxConfigGenerator;
use aether_desktop_lib::settings::SettingsStorage;

fn main() {
    let mut settings = SettingsStorage::load();
    if std::env::var_os("AETHER_DUMP_MAC_PROFILE").is_some() { settings.sing_box.interface_name="utun99".into();settings.aether.manual_profile=Some(aether_desktop_lib::models::profile::ConnectionProfile::PsiphonAuto); }
    let config = SingBoxConfigGenerator::generate(&settings);
    let json = SingBoxConfigGenerator::to_json_string(&config).unwrap();
    let path = SettingsStorage::get_singbox_config_path();
    std::fs::write(&path, json).unwrap();
    println!("sing-box config successfully generated to: {:?}", path);
}
