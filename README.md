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
  --capabilities-ttl-seconds 15 \
  --probe-diagnostics \
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

## Runtime Detection Capabilities

Возможности плеера определяются в рантайме на основе:

- MPRIS metadata (`canPlay`, `canPause`, `canGoNext`, `canGoPrevious`, `canSeek`, `canStop`);
- безопасных probe-запросов (`playerctl volume`, `shuffle`, `loop`, шаблоны metadata);
- fallback-логики без destructive side-effects.

Для `can_stop` используется безопасная эвристика: если `canStop` недоступен, значение выводится из `can_play`/`can_pause` и текущего `status`, без отправки команды `stop` как теста.

## Capabilities Cache TTL

Чтобы не дергать `playerctl` на каждом тике, `capabilities` кэшируются:

- пересчет по TTL (`--capabilities-ttl-seconds`, по умолчанию `15`);
- пересчет при смене активного плеера;
- публикация в `.../capabilities` только при фактическом изменении payload.

Это снижает шум и нагрузку на D-Bus/MPRIS.

## Probe Diagnostics

При флаге `--probe-diagnostics` адаптер публикует диагностический отчет в `.../event`:

- какие проверки выполнены;
- какой источник использован (`metadata`, `query`, `probe`, `fallback`);
- почему конкретная capability получилась `true` или `false`.

Сообщения отправляются только при изменении диагностического payload, чтобы не создавать лишний шум.

Пример event (успешный probe):

```json
{
  "status": "capabilities-probe",
  "player_selector": "playerctld,%any",
  "resolved_player": "spotify",
  "fallback": false,
  "checks": [
    {
      "capability": "can_seek",
      "passed": true,
      "source": "metadata",
      "reason": "{{mpris:canSeek}}=true"
    }
  ],
  "capabilities": {
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
}
```

Пример event (плеер недоступен):

```json
{
  "status": "capabilities-probe",
  "player_selector": "playerctld,%any",
  "resolved_player": null,
  "fallback": true,
  "checks": [
    {
      "capability": "can_control",
      "passed": false,
      "source": "status",
      "reason": "status query failed: playerctl [\"status\"] failed: No players found"
    }
  ],
  "capabilities": {
    "can_play": false,
    "can_pause": false,
    "can_stop": false,
    "can_next": false,
    "can_previous": false,
    "can_seek": false,
    "can_set_volume": false,
    "can_shuffle": false,
    "can_loop": false
  }
}
```

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
