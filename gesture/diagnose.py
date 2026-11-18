"""Count reports from the HONOR I²C touchpad and its virtual keyboard for 35 seconds."""
import collections
import glob
import os
from pathlib import Path
import selectors
import struct
import time

virt_name = "HONOR MagicBook Art Gesture Keys"
event_size = struct.calcsize("@llHHi")
virtual_nodes = [
    f"/dev/input/{Path(path).parent.parent.name}"
    for path in glob.glob("/sys/class/input/event*/device/name")
    if Path(path).read_text().strip() == virt_name
]
if len(virtual_nodes) != 1:
    raise SystemExit(f"Expected one virtual gesture device, found {len(virtual_nodes)}")
selector = selectors.DefaultSelector()
report_counts = collections.Counter()
gesture_counts = collections.Counter()
key_counts = collections.Counter()
try:
    for path, kind in (("/dev/honor-mra-touchpad", "hid"), (virtual_nodes[0], "virtual")):
        fd = os.open(path, os.O_RDONLY | os.O_NONBLOCK | os.O_CLOEXEC)
        selector.register(fd, selectors.EVENT_READ, kind)
    print("READY. Проведите вверх и вниз вдоль обоих краёв, затем дважды стукните костяшкой.", flush=True)
    deadline = time.monotonic() + 35
    while time.monotonic() < deadline:
        for key, _ in selector.select(max(0, deadline - time.monotonic())):
            try:
                data = os.read(key.fd, 64 if key.data == "hid" else event_size * 16)
            except BlockingIOError:
                continue
            if not data:
                raise SystemExit(f"{key.data} disconnected")
            if key.data == "hid":
                report_counts[(hex(data[0]), len(data))] += 1
                if data[0] == 0x0E and len(data) >= 3:
                    gesture_counts[(hex(data[1]), hex(data[2]))] += 1
            else:
                for _, _, event_type, code, value in struct.iter_unpack("@llHHi", data):
                    if event_type == 1 and value == 1:
                        key_counts[code] += 1
finally:
    for device in list(selector.get_map().values()):
        os.close(device.fd)
    selector.close()
    print(f"HID report ID/length: {dict(report_counts)}")
    print(f"HONOR 0x0e gesture/direction: {dict(gesture_counts)}")
    print(f"Virtual key presses by code: {dict(key_counts)}")
