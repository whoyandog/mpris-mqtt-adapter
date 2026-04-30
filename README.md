# mpris-mqtt-adapter

Легковесный адаптер между MPRIS (через `playerctl`) и MQTT.

Адаптер:
- публикует текущее состояние плеера в MQTT;
- принимает команды управления из MQTT;
- (опционально) публикует Home Assistant MQTT Discovery для сенсоров/кнопок;
- публикует `capabilities`, определяя возможности плеера в рантайме (probe + metadata).

## Требования

- Linux с D-Bus и MPRIS-совместимым плеером
- `playerctl` в `PATH`
- MQTT-брокер (например, Mosquitto)

## Сборка

```bash
cargo build --release
```

Бинарник:

```bash
./target/release/mpris-mqtt-adapter
```

## Запуск

```bash
./target/release/mpris-mqtt-adapter \
  --host mqtt.home.arpa \
  --port 1883 \
  --topic workstation/media \
  --player "playerctld,%any" \
  --poll-seconds 2 \
  --discovery
```

Переменные окружения (опционально):

- `MQTT_USERNAME`
- `MQTT_PASSWORD`

## MQTT топики

Для `--topic workstation/media`:

- `workstation/media/state` (retained): JSON состояния
- `workstation/media/capabilities` (retained): JSON возможностей плеера
- `workstation/media/availability` (retained): `online` / `offline`
- `workstation/media/event`: ошибки и сервисные события
- `workstation/media/cmd`: входящие JSON-команды

## Формат state

Пример:

```json
{
  "state": "playing",
  "artist": "Artist",
  "title": "Track",
  "album": "Album",
  "art_url": "https://...",
  "volume": 0.52,
  "position_seconds": 31.2,
  "duration_seconds": 244.0,
  "loop_status": "none",
  "shuffle": "off",
  "player": "spotify"
}
```

## Формат capabilities

Пример:

```json
{
  "can_play": true,
  "can_pause": true,
  "can_stop": true,
  "can_next": true,
  "can_previous": true,
  "can_seek": true,
  "can_set_volume": true,
  "can_shuffle": true,
  "can_loop": true
}
```

## Команды (topic `.../cmd`)

Сообщение должно быть JSON вида:

```json
{ "action": "play_pause" }
```

или

```json
{ "action": "volume_set", "value": 0.5 }
```

Поддерживаемые `action`:

- `play`
- `pause`
- `play_pause` (alias: `toggle`)
- `next`
- `previous` (alias: `prev`)
- `stop`
- `volume_set` (`value`: number)
- `volume_up` (`value`: number, опционально)
- `volume_down` (`value`: number, опционально)
- `mute`
- `position_set` (`value`: seconds)
- `position_seek` (`value`: delta seconds)
- `loop_none`
- `loop_track`
- `loop_playlist`
- `shuffle_on`
- `shuffle_off`

## Home Assistant Discovery

При флаге `--discovery` публикуются конфиги MQTT Discovery для:

- сенсоров (state/title/artist/album/art_url);
- кнопок (play_pause/next/previous/stop).

## Лицензия

MIT
