// Замена QuadFiles (macroquad load_file/load_string).
// Десктоп: std::fs, рабочая директория = dt/ (как у quad_ui — build.sh кладёт
// бинарник в dt/). Поведение совпадает: отсутствующий файл -> panic.
// Android: ассеты APK через AAssetManager (android-activity).

/// Глобальный AndroidApp — заполняется в android_main (runtime-инпут), читается
/// файловым слоем.
#[cfg(target_os = "android")]
pub static ANDROID_APP: std::sync::OnceLock<android_activity::AndroidApp> =
    std::sync::OnceLock::new();

pub struct DesktopFiles;

impl dt_lib::parse::FileAccess for DesktopFiles {
    async fn read(path: &str) -> Vec<u8> {
        #[cfg(not(target_os = "android"))]
        return std::fs::read(path)
            .unwrap_or_else(|e| panic!("file read failed: {path}: {e}"));
        #[cfg(target_os = "android")]
        {
            use std::io::Read;
            let app = ANDROID_APP
                .get()
                .expect("android app not initialized");
            let am = app.asset_manager();
            let mut asset = am
                .open(path)
                .unwrap_or_else(|e| panic!("apk asset open failed: {path}: {e:?}"));
            let mut out = Vec::new();
            asset.read_to_end(&mut out).expect("asset read failed");
            out
        }
    }
}

/// Синхронное чтение файла (шрифты/иконки вне FileAccess-пайплайна).
pub fn read_file_bytes(path: &str) -> Vec<u8> {
    #[cfg(not(target_os = "android"))]
    return std::fs::read(path)
        .unwrap_or_else(|e| panic!("file read failed: {path}: {e}"));
    #[cfg(target_os = "android")]
    {
        use std::io::Read;
        let app = ANDROID_APP.get().expect("android app not initialized");
        let am = app.asset_manager();
        let mut asset = am
            .open(path)
            .unwrap_or_else(|e| panic!("apk asset open failed: {path}: {e:?}"));
        let mut out = Vec::new();
        asset.read_to_end(&mut out).expect("asset read failed");
        out
    }
}
