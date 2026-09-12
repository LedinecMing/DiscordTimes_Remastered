// Discord Rich Presence: статус игры в Discord у игрока.
// Подключение к локальному IPC-сокету Discord; игра не зависит от него —
// все ошибки глотаются с логом в stderr (RPC не обязателен для игры).
// Кнопки: «Discord игры» (инвайт на сервер); для онлайн-комнат —
// «Присоединиться/Смотреть» по секрету комнаты (комнаты появятся с PVP).
use discord_rich_presence::{
    activity::{Activity, Assets, Button, Secrets, Timestamps},
    DiscordIpc, DiscordIpcClient,
};

/// Application ID Discord-приложения игры.
/// Переопределяется переменной окружения DT_DISCORD_APP_ID.
const DEFAULT_APP_ID: &str = "1141007311771541524";

/// Инвайт на Discord-сервер игры (кнопка на всех статусах).
const INVITE_URL: &str = "https://discord.gg/UdnHjUMMGc";

/// Что показывать в статусе.
#[derive(Debug, Clone, PartialEq)]
pub enum PresenceState {
    /// Главное меню / вне игры.
    Menu,
    /// Карта сценария: сценарий/карта, золото, идёт бой и с кем, победы подряд.
    Map {
        scenario: String,
        gold: u64,
        in_battle: bool,
        enemy: Option<String>,
        recent_wins: u32,
    },
    /// Онлайн-комната: код комнаты для join/spectate.
    /// PVP-комнат ещё нет — стейт пока не строится, оставлен для будущего.
    Room {
        id: String,
        join_secret: String,
        spectate_secret: String,
        in_battle: bool,
    },
    /// Лобби ПВП (в комнате, бой не начался). См. Room.
    RoomLobby {
        id: String,
        join_secret: String,
        spectate_secret: String,
    },
}

/// Клиент Discord IPC. Если Discord не запущен, клиент остаётся `None` —
/// все методы no-op, игра работает без статуса.
pub struct RichPresence {
    client: Option<DiscordIpcClient>,
    /// Unix-время старта (мс) — «играет N минут» в статусе.
    started_at: i64,
    /// Dedup-ключ последнего отправленного стейта.
    last_state: Option<String>,
}

impl std::fmt::Debug for RichPresence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RichPresence")
            .field("connected", &self.client.is_some())
            .field("started_at", &self.started_at)
            .finish()
    }
}

impl Default for RichPresence {
    fn default() -> Self {
        Self::new()
    }
}

impl RichPresence {
    /// Подключиться к Discord (если запущен). Отказ быстрый и тихий:
    /// UnixStream::connect к отсутствующему сокету падает мгновенно,
    /// блокировки потока нет.
    pub fn new() -> Self {
        let app_id =
            std::env::var("DT_DISCORD_APP_ID").unwrap_or_else(|_| DEFAULT_APP_ID.into());
        let mut client = DiscordIpcClient::new(&app_id);
        let client = match client.connect() {
            Ok(()) => {
                eprintln!("[rpc] discord подключён (app {app_id})");
                Some(client)
            }
            Err(e) => {
                // Нет IPC-сокета / Discord не запущен — штатный случай.
                eprintln!("[rpc] discord не запущен, RPC выключен: {e}");
                None
            }
        };
        Self {
            client,
            started_at: now_unix_ms(),
            last_state: None,
        }
    }

    /// Обновить статус (идемпотентно: IPC трогаем только при изменении).
    pub fn update(&mut self, state: &PresenceState) {
        let Some(client) = self.client.as_mut() else {
            return;
        };
        let key = format!("{state:?}");
        if self.last_state.as_deref() == Some(key.as_str()) {
            return; // ничего не изменилось — не спамим IPC
        }
        self.last_state = Some(key);
        if let Err(e) = client.set_activity(build_activity(state, self.started_at)) {
            // Discord мог закрыться — один retry с переподключением.
            eprintln!("[rpc] update не прошёл ({e}), переподключаюсь");
            match client.reconnect() {
                Ok(()) => {
                    if let Err(e) = client.set_activity(build_activity(state, self.started_at)) {
                        eprintln!("[rpc] update после переподключения не прошёл: {e}");
                    }
                }
                Err(e) => eprintln!("[rpc] переподключение не удалось: {e}"),
            }
        }
    }

    /// Скрыть статус (выход из игры).
    pub fn clear(&mut self) {
        if let Some(client) = self.client.as_mut() {
            if let Err(e) = client.clear_activity() {
                eprintln!("[rpc] clear не прошёл: {e}");
            }
        }
    }
}

impl Drop for RichPresence {
    fn drop(&mut self) {
        self.clear();
    }
}

fn build_activity(state: &PresenceState, started_at: i64) -> Activity<'static> {
    let timestamps = Timestamps::new().start(started_at);
    let assets = Assets::new()
        .large_image("shlem") // ключ ассета, загруженного в Discord Developer Portal
        .large_text("Времена Раздора: Remastered");
    match state {
        PresenceState::Menu => Activity::new()
            .details("В главном меню")
            .assets(assets)
            .timestamps(timestamps)
            .buttons(vec![Button::new("Discord игры", INVITE_URL)]),
        PresenceState::Map {
            scenario,
            gold,
            in_battle,
            enemy,
            recent_wins,
        } => {
            let details = if *in_battle {
                match enemy {
                    Some(name) => format!("Сражается против {name}"),
                    None => "Сражается".to_string(),
                }
            } else {
                format!("Сценарий: {scenario}")
            };
            let mut state_line = format!("Золото: {gold}");
            if *recent_wins > 0 {
                state_line.push_str(&format!(" · Побед подряд: {recent_wins}"));
            }
            Activity::new()
                .details(details)
                .state(state_line)
                .assets(assets)
                .timestamps(timestamps)
                .buttons(vec![Button::new("Discord игры", INVITE_URL)])
        }
        PresenceState::RoomLobby {
            id,
            join_secret,
            spectate_secret,
        } => Activity::new()
            .details("В лобби комнаты")
            .state(format!("Комната {id}"))
            .assets(assets)
            .timestamps(timestamps)
            .secrets(
                Secrets::new()
                    .join(join_secret.clone())
                    .spectate(spectate_secret.clone()),
            )
            .buttons(vec![
                Button::new("Discord игры", INVITE_URL),
                Button::new("Смотреть", format!("{INVITE_URL}#{spectate_secret}")),
            ]),
        PresenceState::Room {
            id,
            join_secret,
            spectate_secret,
            in_battle,
        } => {
            let details = if *in_battle {
                "Играет в ПВП"
            } else {
                "В комнате"
            };
            Activity::new()
                .details(details)
                .state(format!("Комната {id}"))
                .assets(assets)
                .timestamps(timestamps)
                .secrets(
                    Secrets::new()
                        .join(join_secret.clone())
                        .spectate(spectate_secret.clone()),
                )
                .buttons(vec![
                    Button::new("Discord игры", INVITE_URL),
                    Button::new("Присоединиться", format!("{INVITE_URL}#{join_secret}")),
                ])
        }
    }
}

fn now_unix_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Activity сериализуется в JSON без подключения к Discord —
    /// поля приватные, поэтому проверяем через serde_json.
    fn json(state: &PresenceState) -> serde_json::Value {
        serde_json::to_value(build_activity(state, 1_700_000_000_000)).unwrap()
    }

    fn map_state(gold: u64, in_battle: bool, enemy: Option<&str>, recent_wins: u32) -> PresenceState {
        PresenceState::Map {
            scenario: "Paramount War".into(),
            gold,
            in_battle,
            enemy: enemy.map(str::to_string),
            recent_wins,
        }
    }

    #[test]
    fn menu_activity() {
        let v = json(&PresenceState::Menu);
        assert_eq!(v["details"], "В главном меню");
        assert_eq!(v["state"], serde_json::Value::Null);
        let buttons = v["buttons"].as_array().unwrap();
        assert_eq!(buttons.len(), 1);
        assert_eq!(buttons[0]["label"], "Discord игры");
        assert_eq!(buttons[0]["url"], INVITE_URL);
        assert_eq!(v["assets"]["large_image"], "shlem");
        assert_eq!(v["assets"]["large_text"], "Времена Раздора: Remastered");
        assert_eq!(v["timestamps"]["start"], 1_700_000_000_000_i64);
    }

    #[test]
    fn map_activity_peaceful() {
        let v = json(&map_state(100, false, None, 0));
        assert_eq!(v["details"], "Сценарий: Paramount War");
        assert_eq!(v["state"], "Золото: 100");
        let buttons = v["buttons"].as_array().unwrap();
        assert_eq!(buttons.len(), 1);
        assert_eq!(buttons[0]["label"], "Discord игры");
    }

    #[test]
    fn map_activity_battle() {
        let v = json(&map_state(100, true, Some("Орда мертвяков"), 0));
        assert_eq!(v["details"], "Сражается против Орда мертвяков");
        assert_eq!(v["state"], "Золото: 100");

        let v = json(&map_state(100, true, None, 0));
        assert_eq!(v["details"], "Сражается");
    }

    #[test]
    fn map_activity_recent_wins() {
        let v = json(&map_state(100, false, None, 3));
        assert_eq!(v["state"], "Золото: 100 · Побед подряд: 3");
        // Нулевой стрик не показывается.
        let v = json(&map_state(100, false, None, 0));
        assert_eq!(v["state"], "Золото: 100");
    }

    #[test]
    fn room_lobby_activity() {
        let v = json(&PresenceState::RoomLobby {
            id: "AB12".into(),
            join_secret: "join-secret".into(),
            spectate_secret: "spec-secret".into(),
        });
        assert_eq!(v["details"], "В лобби комнаты");
        assert_eq!(v["state"], "Комната AB12");
        assert_eq!(v["secrets"]["join"], "join-secret");
        assert_eq!(v["secrets"]["spectate"], "spec-secret");
        let buttons = v["buttons"].as_array().unwrap();
        assert_eq!(buttons.len(), 2);
        assert_eq!(buttons[0]["label"], "Discord игры");
        assert_eq!(buttons[1]["label"], "Смотреть");
        assert_eq!(buttons[1]["url"], format!("{INVITE_URL}#spec-secret"));
    }

    #[test]
    fn room_activity() {
        let v = json(&PresenceState::Room {
            id: "AB12".into(),
            join_secret: "join-secret".into(),
            spectate_secret: "spec-secret".into(),
            in_battle: false,
        });
        assert_eq!(v["details"], "В комнате");
        assert_eq!(v["state"], "Комната AB12");
        let buttons = v["buttons"].as_array().unwrap();
        assert_eq!(buttons.len(), 2);
        assert_eq!(buttons[1]["label"], "Присоединиться");
        assert_eq!(buttons[1]["url"], format!("{INVITE_URL}#join-secret"));

        let v = json(&PresenceState::Room {
            id: "AB12".into(),
            join_secret: "join-secret".into(),
            spectate_secret: "spec-secret".into(),
            in_battle: true,
        });
        assert_eq!(v["details"], "Играет в ПВП");
    }

    /// Dedup-ключ update() различает все значимые изменения стейта —
    /// иначе статус завис бы на устаревшем золоте/стрике.
    #[test]
    fn dedup_key_distinguishes_states() {
        let base = map_state(100, false, None, 0);
        let base_key = format!("{base:?}");
        let different = [
            map_state(101, false, None, 0),                     // золото
            map_state(100, true, None, 0),                      // бой начался
            map_state(100, false, Some("Орда"), 0),             // имя врага
            map_state(100, false, None, 2),                     // стрик
            PresenceState::Menu,
            PresenceState::RoomLobby {
                id: "AB12".into(),
                join_secret: String::new(),
                spectate_secret: String::new(),
            },
        ];
        for state in &different {
            assert_ne!(base_key, format!("{state:?}"), "ключ не различает {state:?}");
        }
        // Одинаковые стейты дают одинаковый ключ — IPC не дёргается каждый кадр.
        assert_eq!(base_key, format!("{:?}", map_state(100, false, None, 0)));
    }
}
