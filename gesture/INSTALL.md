# Служба жестов HONOR MagicBook Art 14

Локальная сборка предназначена для тачпада I²C `35cc:0104` на MRA-XXX.
Служба запускается при обнаружении устройства через udev. Для запуска при
загрузке отдельный вызов `systemctl enable` не нужен. Она открывает hidraw
только на чтение и отправляет клавиши через `/dev/uinput`. Модуль `uinput`
поставляется с текущим ядром Kali.

## Установка после согласования

Команды предназначены для выполнения из корня репозитория после `make gestures test-gestures`. Перед
установкой проверить, что целевые файлы свободны. Все установленные файлы
принадлежат root. Модуль в текущем сеансе загружается до запуска службы.
Локальный `bash gesture/install.sh` выполняет эти действия и откатывает
созданные файлы при ошибке запуска.

```bash
sudo install -o root -g root -m 0755 build/honor-mra-gesture-daemon /usr/local/libexec/honor-mra-gesture-daemon
sudo install -o root -g root -m 0644 gesture/honor-mra-gestures.service /etc/systemd/system/honor-mra-gestures.service
sudo install -o root -g root -m 0644 gesture/70-honor-mra-touchpad.rules /etc/udev/rules.d/70-honor-mra-touchpad.rules
sudo install -o root -g root -m 0644 gesture/honor-mra-uinput.conf /etc/modules-load.d/honor-mra-uinput.conf
sudo modprobe uinput
sudo systemctl daemon-reload
sudo udevadm control --reload-rules
touchpad_path=$(udevadm info --query=path --name=/dev/hidraw1)
sudo udevadm trigger --action=add --settle "/sys$touchpad_path"
sudo systemctl start honor-mra-gestures.service
```

Номер `hidraw1` проверен для текущей загрузки. Перед повторным запуском
команд его следует сверить с `HID_ID=0018:000035CC:00000104` в
`/sys/class/hidraw/hidraw*/device/uevent`.

Проверить статус и журнал. В GNOME провести пальцем вверх и вниз по каждому
краю. Проверить яркость по `/sys/class/backlight/intel_backlight/brightness`,
а громкость через `wpctl get-volume @DEFAULT_AUDIO_SINK@`. Для верхних углов
использовать окно без несохранённых изменений. Двойной стук одной костяшкой
открывает интерфейс скриншота, двумя костяшками делает снимок. После остановки
службы проверить, что она завершилась за три секунды без касания тачпада.
После следующей согласованной перезагрузки повторить проверку автозапуска и жестов.

## Откат

Если установлен модуль настроек, сначала откатите его по инструкции ниже.

```bash
sudo systemctl stop honor-mra-gestures.service
sudo rm -f /etc/udev/rules.d/70-honor-mra-touchpad.rules \
  /etc/systemd/system/honor-mra-gestures.service \
  /etc/modules-load.d/honor-mra-uinput.conf \
  /usr/local/libexec/honor-mra-gesture-daemon
sudo systemctl daemon-reload
sudo udevadm control --reload-rules
touchpad_path=$(udevadm info --query=path --name=/dev/hidraw1)
sudo udevadm trigger --action=change --settle "/sys$touchpad_path"
```

Ссылка в `/dev` пересчитывается при обработке события udev. Загруженный
штатный модуль можно оставить до перезагрузки или выгрузить через
`sudo modprobe -r uinput`, если он был загружен только для этого теста и
его больше никто не использует.

## Настройки через HonorKit

Сначала установите и проверьте службу по шагам выше. Для добавления
настроек к работающей службе выполните из этого репозитория:

```bash
make gestures test-gestures
bash gesture/enable-settings.sh
```

Обновление сохраняет прежний бинарник в
`/var/backups/honorkit-gesture-daemon.pre-settings`, устанавливает
`/usr/local/libexec/honorkit-touchpad-config` и правило Polkit,
затем перезапускает службу. Настройки по умолчанию соответствуют
прежней раскладке. Статус можно проверить через `pkaction` и
`systemctl status honor-mra-gestures.service`.

В HonorKit откройте настройки через кнопку с шестерёнкой. Управляйте
яркостью, громкостью, скриншотами, оконными жестами и уведомлениями.
При желании поменяйте местами края яркости и громкости. Кнопка
«Сохранить жесты» записывает системную конфигурацию после авторизации.
Изменение действует начиная со следующего жеста, обычно в течение секунды.

Для силы отклика выберите слабый, средний или сильный уровень и отдельно
нажмите «Установить силу». Команда поддерживается для MRA-XXX с I²C HID
`35cc:0104`. Прошивка не сообщает текущее значение, поэтому интерфейс
подтверждает отправку команды. Проверьте результат физическим нажатием.
Для возврата к прежнему отклику выберите прежний уровень, если он известен.
Удаление программы само по себе не отправляет другую настройку в тачпад.

Чтобы откатить обновление, восстановите прежний бинарник и удалите новые
компоненты. Если файл конфигурации был создан, удалите и его.

```bash
sudo install -m 0755 /var/backups/honorkit-gesture-daemon.pre-settings \
  /usr/local/libexec/honor-mra-gesture-daemon.new
sudo mv -T /usr/local/libexec/honor-mra-gesture-daemon.new \
  /usr/local/libexec/honor-mra-gesture-daemon
sudo systemctl restart honor-mra-gestures.service
sudo rm -f /usr/local/libexec/honorkit-touchpad-config \
  /usr/share/polkit-1/actions/io.github.no1se.HonorKit.touchpad.policy \
  /etc/honorkit-gestures.conf /var/backups/honorkit-gesture-daemon.pre-settings
```
