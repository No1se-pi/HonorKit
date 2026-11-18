# Системные компоненты HonorKit

Frieren оформляет меню уже установленного GRUB на Kali amd64 с UEFI. Аудиоправка
задействует второй выход встроенных динамиков Realtek ALC256 через штатный драйвер
Intel SOF. Аппаратная проверка аудио выполнена на HONOR MagicBook Art 14
MRA-XXX M1020 с PCI subsystem `1ee7:2059` и codec subsystem `1ee7:2060`.
При воспроизведении сигнал шёл на оба выхода. Правка загрузилась автоматически
после перезагрузки.

## Быстрая установка

Для установки нужны `make`, `python3`, `grub-common`, `grub2-common`,
`efibootmgr` и `fonts-dejavu-core`. Поддерживается проверенная сборка
GRUB `2.14-2+kali1` для Kali amd64, загруженная из
`EFI/kali/grubx64.efi` с внешним `normal.mod` и выключенным Secure Boot.
Оформление проверено на HONOR MagicBook Art 14 MRA-XXX при 3120×2080.

Для установки темы из терминала выполните команду из каталога HonorKit.

```bash
make install-grub-theme
```

Для установки через приложение сначала подготовьте системные компоненты.
Для GUI нужны установленный HonorKit и Polkit в графическом сеансе.
Выполните команду из проверенной копии репозитория.

```bash
make install-system-tools
```

Команда устанавливает вспомогательную программу, файлы компонентов и правило
Polkit. В приложении откройте **Настройки → Системные компоненты**.
Выберите нужный компонент и подтвердите операцию паролем администратора.
Переустановка или удаление каждого компонента доступны независимо.

Те же операции доступны в терминале.

```bash
/usr/local/libexec/honorkit-system status audio
/usr/local/libexec/honorkit-system status grub
sudo /usr/local/libexec/honorkit-system install audio
sudo /usr/local/libexec/honorkit-system install grub
```

Frieren появляется при следующей загрузке в меню выбора Kali или Windows.
Это экран GRUB перед запуском ядра. Plymouth отвечает за следующий экран загрузки.

Если уже установлен прежний вариант Frieren со статичным фоном, удалите его
через HonorKit или командой `remove grub`, затем установите полную тему.

## Почему работает Enter

На проверенном ноутбуке автоматическая загрузка Kali по таймеру проходила,
а ручной выбор того же пункта завершался возвратом в UEFI. В GRUB 2.14
обработка ручного выбора в `grub_show_menu()` вызывает `grub_cls()` перед
выполнением пункта. Ветвь автоматической загрузки запускает пункт без этого
вызова. Удаление отдельного фона Debian само по себе сбой не устранило.
Пропуск вызова `grub_cls()` в ручной ветви восстановил запуск по Enter
на этом ноутбуке. Причина сбоя внутри графического стека и прошивки не установлена.
[Исходный код меню GRUB](https://sources.debian.org/src/grub2/2.14-3/grub-core/normal/menu.c#L1875)

Помощник вычисляет SHA-256 пакетного и загрузочного модулей. В известном
`normal.mod` он заменяет только два байта инструкции `call *%rax` на
два `nop`. Следующий вызов запускает выбранный пункт, как прежде. Ветка таймера,
ядро и параметры загрузки остаются прежними. Помощник проверяет контрольную сумму
результата, поэтому другая сборка требует отдельной проверки.
[Реализация в HonorKit](honorkit-system)

## Что проверяет установщик

- `make grub-theme` собирает три шрифта PF2 из локальных DejaVu. Меню,
  иконки, подсветка и отсчёт остаются частью темы `gfxmenu`.
- Помощник сверяет активную запись UEFI с примонтированным EFI-разделом и
  проверяет, что EFI-образ загружает `normal.mod` с диска. Изменяется только
  внешний модуль в `/boot/grub/x86_64-efi/`. EFI-образ, порядок загрузки UEFI
  и пакетная копия в `/usr/lib/grub` сохраняются.
- `grub-mkconfig` создаёт конфигурацию во временном каталоге. Помощник
  проверяет её через `grub-script-check`, сверяет список и команды загрузки,
  выбор по умолчанию, значения таймера и содержимое секций вне заголовка
  и блока оформления Debian, затем проверяет подключение темы
  и всех трёх шрифтов. Подготовленный `grub.cfg` заменяет текущий одним
  переименованием. При ошибке до замены установщик отменяет свои изменения.
- Исходный модуль, если его исправил HonorKit, хранится в
  `/var/backups/honorkit-grub-normal.mod`. Копия конфигурации создаётся в
  `/var/backups/honorkit-frieren.before-install.cfg` или
  `/var/backups/honorkit-frieren.before-remove.cfg`.

Тема хранится в `/boot/grub/themes/honorkit-frieren-theme`, её настройки в
`/etc/default/grub.d/zzzz-honorkit-frieren.cfg`. После установки проверьте
на ноутбуке два запуска Kali, один по таймеру и один через Enter. Проверка
`grub-script-check` проверяет синтаксис, а аппаратную проверку этих двух
сценариев выполняет загрузка. Обновление пакетов GRUB может заменить внешний
модуль. Если `status grub` показывает `repair`, повторите установку для совместимой
сборки или удалите тему. Правка автоматически на новые сборки не переносится.

Команда `make test-grub-theme` запускает проверки на временных файлах.
Для тестов требуется поддерживаемая сборка GRUB.

## Аудиоправка

Аудиоправка проверяет модель, контроллер SOF, ID кодека и исходную разводку.
Она состоит из двух текстовых файлов:

- `/etc/modprobe.d/90-honor-mra-audio.conf`
- `/usr/lib/firmware/honor-mra-alc256-speakers.fw`

Для проверки после перезагрузки используйте:

```bash
journalctl -k -b -g 'Applying patch firmware|autoconfig for ALC256'
```

На проверенном устройстве в журнале видны `honor-mra-alc256-speakers.fw` и
`line_outs=2 (0x1b/0x14/…)`. Если ядро само настроило второй выход, используйте его штатную конфигурацию.

[Формат конфигурации HD Audio](https://docs.kernel.org/sound/hd-audio/notes.html#early-patching) ·
[Измерения аудио на близкой ревизии с тем же ID кодека](https://github.com/hermeticvm/magicbook14-linux/blob/main/docs/audio.md)

## Удаление и восстановление

Сначала удалите нужный компонент в приложении или выполните:

```bash
sudo /usr/local/libexec/honorkit-system remove grub
sudo /usr/local/libexec/honorkit-system remove audio
```

После удаления компонентов можно удалить помощник и его ресурсы:

```bash
make uninstall-system-tools
```

Помощник восстанавливает исходный `normal.mod`, только если изменил его
сам и файл всё ещё совпадает с исправленной версией. Если исправление уже было
применено вручную или файл заменило обновление GRUB, удаление темы сохраняет этот
модуль.

Для аварийного отката загрузитесь в Kali или откройте её файловую систему из
rescue mode. Команды ниже рассчитаны на установленную Kali или `chroot` в неё.
Сначала проверьте резервную копию конфигурации. Модуль восстанавливайте, только
если с момента установки темы версия GRUB не менялась и текущий файл совпадает
с исправленной сборкой.

```bash
sudo grub-script-check /var/backups/honorkit-frieren.before-install.cfg
if sudo test -f /var/backups/honorkit-grub-normal.mod; then
    sudo sha256sum /boot/grub/x86_64-efi/normal.mod /var/backups/honorkit-grub-normal.mod
fi
```

Если первая контрольная сумма начинается с `81833807` и вторая с
`c062f86c`, а версия GRUB прежняя, восстановите модуль.

```bash
sudo install -m 0644 /var/backups/honorkit-grub-normal.mod /boot/grub/x86_64-efi/normal.mod
```

Затем уберите настройку HonorKit из списка активных и верните конфигурацию.

```bash
sudo mv /etc/default/grub.d/zzzz-honorkit-frieren.cfg /etc/default/grub.d/zzzz-honorkit-frieren.cfg.disabled
sudo install -m 0600 /var/backups/honorkit-frieren.before-install.cfg /boot/grub/grub.cfg
```
